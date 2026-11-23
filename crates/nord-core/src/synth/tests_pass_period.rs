//! The envelope pass period (`STRIP_PASS_FRAMES`) against the firmware display tables, all knobs
//! (D237), and the opt-in `settled_rates` switch.

use super::fw_env_settled::{settled_attack_coeff, settled_decay_release_coeff};
use super::*;
use crate::cbin::presets::{ENV_ATTACK_MS_TABLE, ENV_DECAY_MS_TABLE};

/// Frames per pass that make `passes` passes last `ms` at the DSP frame rate.
fn frames_per_pass(ms: f32, passes: f32) -> f32 {
    ms * 1e-3 * SY_FRAME_RATE_HZ / passes
}

fn median(v: &mut [f32]) -> f32 {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v[v.len() / 2]
}

/// VERIFIED: the settled tables with a 2 frame pass reproduce the displayed attack time (`1 / b`
/// passes) and the displayed 40 dB decay time (`ln 100 / (1 - a)` passes) at all 128 knobs: median
/// within 0.5 %, every knob within 8 % (display rounding). The stale sweep reads up to 5 frames.
#[test]
fn settled_tables_fit_two_frames_per_pass_on_every_knob() {
    let (mut att, mut dec) = (Vec::new(), Vec::new());
    let (mut att_old, mut dec_old) = (Vec::new(), Vec::new());
    for k in 0..128usize {
        let i = k as f32 - 64.0;
        att.push(frames_per_pass(
            ENV_ATTACK_MS_TABLE[k],
            1.0 / settled_attack_coeff(i),
        ));
        dec.push(frames_per_pass(
            ENV_DECAY_MS_TABLE[k],
            100f32.ln() / (1.0 - settled_decay_release_coeff(i)),
        ));
        att_old.push(frames_per_pass(
            ENV_ATTACK_MS_TABLE[k],
            1.0 / attack_coeff(i),
        ));
        dec_old.push(frames_per_pass(
            ENV_DECAY_MS_TABLE[k],
            100f32.ln() / (1.0 - decay_release_coeff(i)),
        ));
    }
    for v in [&att, &dec] {
        assert!(
            v.iter().all(|f| (f / STRIP_PASS_FRAMES - 1.0).abs() < 0.08),
            "{v:?}"
        );
    }
    for v in [&mut att, &mut dec] {
        assert!((median(v) / STRIP_PASS_FRAMES - 1.0).abs() < 0.005);
    }
    for v in [&att_old, &dec_old] {
        assert!(
            v.iter().cloned().fold(0.0, f32::max) > 3.5,
            "stale reads a longer pass"
        );
    }
    assert_eq!(STRIP_PASS_FRAMES, 2.0);
}

fn layer(settled: bool) -> SynthLayer {
    let mut s = SynthLayer::new(48_000.0);
    s.chain = SynthChain::FirmwareStrip;
    s.attack_sec = 0.0123;
    s.decay_sec = 0.37;
    s.sustain_level = 0.0;
    s.release_sec = 0.2;
    s.settled_rates = settled;
    s.knob_laws = true;
    let e = EnvKnobs {
        attack: 24,
        decay: 24,
        release: 4,
    };
    s.strip_knobs = Some(StripKnobs { amp: e, filter: e });
    s
}

#[test]
fn settled_rates_is_off_by_default_and_changes_only_the_rate_tables() {
    assert!(!SynthLayer::new(48_000.0).settled_rates);
    let (mut a, mut b) = (layer(false), layer(true));
    let (pa, pb) = (a.strip_params(), b.strip_params());
    assert_ne!(pa.amp.attack.b.to_bits(), pb.amp.attack.b.to_bits());
    assert_ne!(pa.amp.decay.a.to_bits(), pb.amp.decay.a.to_bits());
    assert_eq!(pa.amp.attack.a, pb.amp.attack.a);
    assert_eq!(pa.period_frames, pb.period_frames);
    // The cache key follows the switch: flipping it on one layer rebuilds the records.
    a.settled_rates = true;
    assert_eq!(
        a.strip_params().amp.attack.b.to_bits(),
        pb.amp.attack.b.to_bits()
    );
}

#[test]
fn settled_rates_matches_the_table_the_adr_law_reads() {
    let mut s = layer(true);
    let mut l = layer(false);
    l.fw_adr_law = true;
    // Same tables; the adr law adds the 0.5 sustain, so compare the attack record only.
    let (ps, pl) = (s.strip_params(), l.strip_params());
    assert_eq!(ps.amp.decay.a.to_bits(), pl.amp.decay.a.to_bits());
    assert_eq!(ps.amp.release.a.to_bits(), pl.amp.release.a.to_bits());
    assert_eq!(ps.filter.decay.a.to_bits(), pl.filter.decay.a.to_bits());
}
