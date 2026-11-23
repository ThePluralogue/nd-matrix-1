//! Response per Mid position of the firmware Amp LP24 / HP24 port against the `sharc-fx` oracle
//! (D234, `tools/fx_lp24_mid_sweep.py`): LP24 and HP24, 9 Mid positions at 3 cutoffs, settled
//! smoothers, 128 calls of seeded LCG noise (amplitude 0.4, seed 7000). The oracle's 12 band power
//! gains (left channel, bins of a 512 point DFT) are in `fx-lp24-mid-sweep.csv`; the port must
//! stay within [`TOLERANCE_DB`] of every band. Run with `--nocapture` for the per position table.

use super::fw_lp24::{FwLp24, Lp24Mode};

const CSV: &str = include_str!("../../../../docs/research/data/fx-lp24-mid-sweep.csv");
/// Band edges in DFT bins (same as the tool).
const EDGES: [usize; 13] = [1, 2, 4, 6, 9, 14, 21, 32, 48, 72, 108, 160, 256];
const CALLS: usize = 128;
const SEED: u32 = 7000;
/// Tolerance of the band gains, dB (VERIFIED against the oracle on all 54 scenarios, D234).
const TOLERANCE_DB: f64 = 0.01;

fn lcg_frames(seed: u32, n: usize) -> Vec<[f32; 2]> {
    let mut state = seed;
    let mut next = || {
        state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (((f64::from(state >> 8) / f64::from(1u32 << 24)) - 0.5) * 2.0 * 0.4) as f32
    };
    (0..n).map(|_| [next(), next()]).collect()
}

/// Power of bins `1..256` of the real DFT of `x` (512 samples).
fn power(x: &[f32]) -> Vec<f64> {
    let n = x.len();
    (0..=n / 2)
        .map(|k| {
            let (mut re, mut im) = (0.0f64, 0.0f64);
            for (t, v) in x.iter().enumerate() {
                let a = -2.0 * std::f64::consts::PI * (k * t % n) as f64 / n as f64;
                re += f64::from(*v) * a.cos();
                im += f64::from(*v) * a.sin();
            }
            re * re + im * im
        })
        .collect()
}

fn bands_db(out: &[f64], inp: &[f64]) -> Vec<f64> {
    EDGES
        .windows(2)
        .map(|e| {
            let y: f64 = out[e[0]..e[1]].iter().sum();
            let x: f64 = inp[e[0]..e[1]].iter().sum();
            10.0 * (y.max(1e-30) / x).log10()
        })
        .collect()
}

/// Band gains (dB) of the port for one scenario.
fn scenario(mode: u8, freq: u8, mid: i8, inp_power: &[f64], frames: &[[f32; 2]]) -> Vec<f64> {
    let m = if mode == 0 {
        Lp24Mode::Low
    } else {
        Lp24Mode::High
    };
    let mut st = FwLp24::new(m, f32::from(freq), f32::from(mid));
    let left: Vec<f32> = frames
        .as_chunks::<4>()
        .0
        .iter()
        .flat_map(|b| st.block(*b).into_iter().map(|f| f[0]))
        .collect();
    bands_db(&power(&left), inp_power)
}

#[test]
fn every_mid_position_matches_the_oracle_response() {
    let frames = lcg_frames(SEED, CALLS * 4);
    let inp: Vec<f32> = frames.iter().map(|f| f[0]).collect();
    let inp_power = power(&inp);
    let mut worst_all = 0.0f64;
    let mut rows = 0;
    println!("mode freq mid | worst band err dB | peak band dB");
    for line in CSV.lines().skip(1) {
        let c: Vec<&str> = line.split(',').collect();
        let (mode, freq, mid): (u8, u8, i8) = (
            c[0].parse().unwrap(),
            c[1].parse().unwrap(),
            c[2].parse().unwrap(),
        );
        let want: Vec<f64> = c[4..16].iter().map(|v| v.parse().unwrap()).collect();
        let got = scenario(mode, freq, mid, &inp_power, &frames);
        let worst = got
            .iter()
            .zip(&want)
            .map(|(g, w)| (g - w).abs())
            .fold(0.0, f64::max);
        let peak = want.iter().cloned().fold(f64::MIN, f64::max);
        println!("{mode:4} {freq:4} {mid:4} | {worst:.5} | {peak:+.2}");
        worst_all = worst_all.max(worst);
        rows += 1;
    }
    assert_eq!(rows, 54, "9 positions x 3 cutoffs x 2 modes");
    assert!(worst_all < TOLERANCE_DB, "worst band error {worst_all} dB");
}

#[test]
fn the_sweep_covers_a_real_range_of_responses() {
    // Not a trivially flat sweep: the peak band gain spans more than 20 dB over the Mid knob.
    let peaks: Vec<f64> = CSV
        .lines()
        .skip(1)
        .map(|l| {
            l.split(',')
                .skip(4)
                .map(|v| v.parse::<f64>().unwrap())
                .fold(f64::MIN, f64::max)
        })
        .collect();
    let span = peaks.iter().cloned().fold(f64::MIN, f64::max)
        - peaks.iter().cloned().fold(f64::MAX, f64::min);
    assert!(span > 20.0, "span {span}");
}
