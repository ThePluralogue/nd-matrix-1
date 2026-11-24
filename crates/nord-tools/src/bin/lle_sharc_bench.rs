//! `lle_sharc_bench` (bean `ns4-lle3`): multi-core SHARC pass benchmark on the lock-free
//! 1-thread-per-core spin-barrier scheduler (`nord_sharc::runner::pass_sched`).
//!
//! ```text
//! cargo run --release -p nord-tools --bin lle_sharc_bench -- \
//!   --core NAME=IMAGE,PRESET[,MSGFILE[,AOT_TAG]]  ...  [--passes 300] [--warmup 16] [--engine interp|aot]
//! ```
//!
//! `PRESET` is `pno` or `org` (`runner::live_presets`), `MSGFILE` a text of `--msg [PASS:]W1=W2`
//! tokens (as `sharc-sys` takes them), `AOT_TAG` the embedded whole-pass module of the core
//! (`NORD_SHARC_AOT_HOT=TAG=file;...` at build time). With `--engine aot` only cores that have a
//! tag run the AOT blocks; the others stay on the interpreter. Output: one row per core and the
//! scheduler tick, all against the 46.08 us pass period (2 frames at 43 402.34 Hz).
use nord_sharc::runner::live_presets::{boot_core, preset_named};
use nord_sharc::runner::msg::{parse_msg, Msg};
use nord_sharc::runner::pass::PassConfig;
use nord_sharc::runner::pass_sched::{run_ticks, summarize, Engine, PassJob};
use nord_sharc::runner::periph_run::PeriphOpts;
use nord_sharc::runner::system::{pass_period_s, FRAME_RATE};
use std::fmt::Write as _;

struct CoreSpec {
    name: String,
    image: String,
    preset: String,
    msgs: Option<String>,
    tag: Option<String>,
}

/// `NAME=IMAGE,PRESET[,MSGFILE[,TAG]]`.
fn parse_core(s: &str) -> Result<CoreSpec, String> {
    let bad = || format!("bad --core {s:?} (want NAME=IMAGE,PRESET[,MSGFILE[,TAG]])");
    let (name, rest) = s.split_once('=').ok_or_else(bad)?;
    let f: Vec<&str> = rest.split(',').collect();
    if f.len() < 2 || f.len() > 4 {
        return Err(bad());
    }
    let opt = |i: usize| f.get(i).filter(|v| !v.is_empty()).map(|v| v.to_string());
    Ok(CoreSpec {
        name: name.into(),
        image: f[0].into(),
        preset: f[1].into(),
        msgs: opt(2),
        tag: opt(3),
    })
}

/// Messages of a text of `--msg X` tokens.
fn parse_msgs(text: &str) -> Result<Vec<Msg>, String> {
    let mut out = Vec::new();
    let mut it = text.split_whitespace();
    while let Some(t) = it.next() {
        if t == "--msg" {
            out.push(parse_msg(it.next().ok_or("--msg without a value")?)?);
        }
    }
    Ok(out)
}

fn job(spec: &CoreSpec, engine: Engine) -> Result<PassJob, String> {
    let data = std::fs::read(&spec.image).map_err(|e| format!("{}: {e}", spec.image))?;
    let preset = preset_named(&spec.preset).ok_or_else(|| format!("preset {:?}", spec.preset))?;
    let mut core = boot_core(&data, &preset, &PeriphOpts::default())?;
    let mut engine = engine;
    match (&spec.tag, engine) {
        (Some(tag), Engine::Aot) => {
            core.enable_aot_tag(tag)?;
        }
        _ => engine = Engine::Interp,
    }
    let msgs = match &spec.msgs {
        Some(f) => parse_msgs(&std::fs::read_to_string(f).map_err(|e| format!("{f}: {e}"))?)?,
        None => Vec::new(),
    };
    Ok(PassJob {
        name: spec.name.clone(),
        core,
        cfg: PassConfig {
            trigger: preset.trigger,
            idle_pc: preset.idle_pc,
            max_steps: 2_000_000,
            irq: preset.irq,
        },
        msgs,
        msg_base: 0,
        ping_pong: preset.ping_pong,
        engine,
    })
}

fn main() -> Result<(), String> {
    let (mut specs, mut passes, mut warmup, mut engine) =
        (Vec::new(), 300u32, 16u32, Engine::Interp);
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        let mut val = || args.next().ok_or(format!("{a} needs a value"));
        match a.as_str() {
            "--core" => specs.push(parse_core(&val()?)?),
            "--passes" => passes = val()?.parse().map_err(|e| format!("--passes: {e}"))?,
            "--warmup" => warmup = val()?.parse().map_err(|e| format!("--warmup: {e}"))?,
            "--engine" => {
                engine = match val()?.as_str() {
                    "interp" => Engine::Interp,
                    "aot" => Engine::Aot,
                    o => return Err(format!("--engine {o:?}")),
                }
            }
            _ => return Err(format!("unknown argument {a:?}")),
        }
    }
    if specs.is_empty() {
        return Err("usage: lle_sharc_bench --core NAME=IMAGE,PRESET[,MSGFILE[,TAG]] ...".into());
    }
    let jobs = specs
        .iter()
        .map(|s| job(s, engine))
        .collect::<Result<Vec<_>, _>>()?;
    let r = run_ticks(jobs, warmup, passes)?;
    let budget_us = pass_period_s(FRAME_RATE) * 1e6;
    let mut out = String::new();
    let _ = writeln!(
        out,
        "engine={engine:?} passes={passes} warmup={warmup} budget={budget_us:.2} us/pass ({} threads)",
        r.cores.len()
    );
    let _ = writeln!(
        out,
        "core | engine | idle | steps/pass | mean us | p50 us | p99 us | worst us | mean/budget"
    );
    let mut total_mean = 0.0;
    for (c, j) in r.cores.iter().zip(&r.jobs) {
        let (mean, med, p99, worst) = summarize(&c.ns, warmup as usize);
        let steps = summarize(&c.steps, warmup as usize).0;
        total_mean += mean;
        let _ = writeln!(
            out,
            "{} | {:?} | {} | {:.0} | {:.1} | {:.1} | {:.1} | {:.1} | x{:.2}",
            c.name,
            j.engine,
            c.all_idle,
            steps,
            mean / 1e3,
            med / 1e3,
            p99 / 1e3,
            worst / 1e3,
            mean / 1e3 / budget_us
        );
    }
    let (mean, med, p99, worst) = summarize(&r.tick_ns, warmup as usize);
    let _ = writeln!(
        out,
        "tick wall (slowest core + barrier): mean {:.1} us, p50 {:.1}, p99 {:.1}, worst {:.1} | total CPU per pass {:.1} us = {:.2} host cores of real time",
        mean / 1e3, med / 1e3, p99 / 1e3, worst / 1e3, total_mean / 1e3, total_mean / 1e3 / budget_us
    );
    print!("{out}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_specs_parse_with_optional_msgs_and_tag() {
        let c = parse_core("A=img.bin,org,m.txt,TAG").unwrap();
        assert_eq!(
            (c.name.as_str(), c.image.as_str(), c.preset.as_str()),
            ("A", "img.bin", "org")
        );
        assert_eq!(
            (c.msgs.as_deref(), c.tag.as_deref()),
            (Some("m.txt"), Some("TAG"))
        );
        let c = parse_core("B=img.bin,pno").unwrap();
        assert!(c.msgs.is_none() && c.tag.is_none());
        let c = parse_core("C=img.bin,org,,TAG").unwrap();
        assert!(c.msgs.is_none() && c.tag.as_deref() == Some("TAG"));
        assert!(parse_core("nope").is_err() && parse_core("D=only").is_err());
    }

    #[test]
    fn msg_tokens_are_read_from_a_sharc_sys_style_text() {
        let m = parse_msgs("--msg 0:0x3e021=1\n--msg 2:0xe02f8=f1.0").unwrap();
        assert_eq!(m.len(), 2);
        assert_eq!((m[0].pass, m[0].word1, m[0].word2), (0, 0x3e021, 1));
        assert_eq!((m[1].pass, m[1].word2), (2, 0x3F80_0000));
        assert!(parse_msgs("--msg").is_err());
        assert!(parse_msgs("--other x").unwrap().is_empty());
    }
}
