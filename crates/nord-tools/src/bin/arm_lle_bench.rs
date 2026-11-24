//! ARM0 LLE benchmark (`ns4-lle2`): boot the user's OS v1.66 ARM image with panel, I2C EEPROM
//! (program memory) and UART2 MIDI in the native Rust core, then time 1 ms ticks through the same
//! phases as `tools/arm_org_stream.py` (load, idle, one key, eight keys), with idle skipping and
//! with full execution on a clone, comparing the machine state hash after every phase.
//!
//!     cargo run --release -p nord-tools --bin arm-lle-bench -- \
//!         <bank0_CODE.bin> <bank0_SRAM.bin> <eeprom_image_256k.bin>
//!
//! Environment: `PHASE_SCALE` (percent of the Python tick counts, default 100), `NO_FULL=1` skips
//! the full-execution comparison.

use nord_arm::machine::idle::IdleStats;
use nord_arm::machine::uart::{note_off, note_on};
use nord_arm::machine::{Machine, Stop};
use std::time::Instant;

/// ASSUMED: ARM instructions per 1 ms tick (`tools/arm_steady.py` INSNS_PER_TICK).
const TICK: u64 = 200_000;
const KEYS8: [u8; 8] = [60, 62, 64, 65, 67, 69, 71, 72];

/// Phase name, tick count, action run on its first tick.
type PhasePlan<'a> = (&'static str, usize, &'a dyn Fn(&mut Machine));

struct Row {
    name: &'static str,
    ms: Vec<f64>,
    executed: u64,
    msgs: usize,
}

fn percentile(v: &[f64], p: f64) -> f64 {
    let mut s = v.to_vec();
    s.sort_by(|a, b| a.partial_cmp(b).unwrap());
    s[((s.len() - 1) as f64 * p) as usize]
}

fn run_phase(
    m: &mut Machine,
    name: &'static str,
    ticks: usize,
    idle: bool,
    stats: &mut IdleStats,
    action: &dyn Fn(&mut Machine),
) -> Row {
    let mut ms = Vec::with_capacity(ticks);
    let before = stats.executed;
    let mut msgs = 0;
    for i in 0..ticks {
        if i == 0 {
            action(m);
        }
        let t = Instant::now();
        let s = if idle {
            m.run_tick_idle(TICK, stats)
        } else {
            m.run_tick(TICK)
        };
        ms.push(t.elapsed().as_secs_f64() * 1e3);
        assert!(
            matches!(s, Stop::Budget),
            "{name} tick {i}: {s:?} pc={:#x}",
            m.cpu.pc()
        );
        msgs += m.drain_messages().len();
    }
    Row {
        name,
        ms,
        executed: stats.executed - before,
        msgs,
    }
}

fn phases(
    m: &mut Machine,
    idle: bool,
    scale: usize,
    stats: &mut IdleStats,
) -> (Vec<Row>, Vec<u64>) {
    let n = |x: usize| (x * scale / 100).max(1);
    let mut rows = Vec::new();
    let mut hashes = Vec::new();
    let none = |_: &mut Machine| {};
    let on1 = |m: &mut Machine| m.send_midi(&note_on(KEYS8[0], 100, 0));
    let off1 = |m: &mut Machine| m.send_midi(&note_off(KEYS8[0], 0));
    let on8 = |m: &mut Machine| KEYS8.iter().for_each(|k| m.send_midi(&note_on(*k, 100, 0)));
    let off8 = |m: &mut Machine| KEYS8.iter().for_each(|k| m.send_midi(&note_off(*k, 0)));
    let plan: [PhasePlan; 6] = [
        ("load", n(800), &none),
        ("idle", n(300), &none),
        ("key1+off", n(40), &on1),
        ("key1 off", n(40), &off1),
        ("key8", n(40), &on8),
        ("key8 off", n(40), &off8),
    ];
    for (name, ticks, action) in plan {
        rows.push(run_phase(m, name, ticks, idle, stats, action));
        hashes.push(m.state_hash());
    }
    (rows, hashes)
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let code = std::fs::read(&a[1]).expect("code");
    let sram = std::fs::read(&a[2]).expect("sram");
    let eeprom = std::fs::read(&a[3]).expect("eeprom image");
    let scale: usize = std::env::var("PHASE_SCALE")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(100);
    let mut m = Machine::new();
    let t = Instant::now();
    m.prepare_system(&code, &sram).expect("init array");
    m.bus.i2c.as_mut().unwrap().load_image(&eeprom);
    let mut boot_stats = IdleStats::default();
    let (n, irqs, stop) = m.boot_system(1_500_000_000, &mut boot_stats);
    println!(
        "boot: {stop:?} pc={:#x} {n} virtual insns ({} executed, {} skipped), {irqs} wait-loop irqs, {:.2}s wall",
        m.cpu.pc(),
        boot_stats.executed,
        boot_stats.skipped,
        t.elapsed().as_secs_f64()
    );
    m.install_dsp_ports();
    m.install_dsp_return();
    m.enable_real_timer();
    let mut full = m.clone();
    let mut stats = IdleStats::default();
    let (rows, h_idle) = phases(&mut m, true, scale, &mut stats);
    println!("\nphase      ticks  mean ms  p50     p99     max     >1ms   exec insns/tick  msgs");
    for r in &rows {
        let mean = r.ms.iter().sum::<f64>() / r.ms.len() as f64;
        let over = r.ms.iter().filter(|x| **x > 1.0).count();
        println!(
            "{:<10} {:>5}  {:>7.3}  {:>6.3}  {:>6.3}  {:>6.3}  {:>4.1}%  {:>12.0}  {:>6}",
            r.name,
            r.ms.len(),
            mean,
            percentile(&r.ms, 0.5),
            percentile(&r.ms, 0.99),
            percentile(&r.ms, 1.0),
            100.0 * over as f64 / r.ms.len() as f64,
            r.executed as f64 / r.ms.len() as f64,
            r.msgs
        );
    }
    println!("idle stats: {stats:?}");
    if std::env::var("NO_FULL").is_err() {
        let mut fs = IdleStats::default();
        let (frows, h_full) = phases(&mut full, false, scale, &mut fs);
        for ((r, f), (a, b)) in rows.iter().zip(&frows).zip(h_idle.iter().zip(&h_full)) {
            let mi = r.ms.iter().sum::<f64>() / r.ms.len() as f64;
            let mf = f.ms.iter().sum::<f64>() / f.ms.len() as f64;
            println!(
                "{:<10} full {:.3} ms/tick, idle {:.3} ms/tick, state hash {}, msgs {} vs {}",
                r.name,
                mf,
                mi,
                if a == b { "EQUAL" } else { "DIFFERENT" },
                f.msgs,
                r.msgs
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percentile_picks_sorted_rank() {
        let v = [5.0, 1.0, 3.0, 2.0, 4.0];
        assert_eq!(percentile(&v, 0.0), 1.0);
        assert_eq!(percentile(&v, 0.5), 3.0);
        assert_eq!(percentile(&v, 1.0), 5.0);
    }
}
