//! `FirmwareFx::lp24` on `LayerFxChain`: the firmware pair replaces the engine's biquads for the
//! `LP24` / `HP24` amps, 4 frame latency, opt-in (the default keeps the engine path).

use super::*;

const LP24_ONLY: FirmwareFx = FirmwareFx {
    lp24: true,
    ..FirmwareFx::OFF
};

fn chain(cab: AmpCabinet, fw: FirmwareFx) -> LayerFxChain {
    let mut fx = LayerFxChain::new(48_000.0);
    fx.reverb_model = ReverbModel::Off;
    fx.amp_cabinet = cab;
    fx.eq_mid_freq_hz = 1000.0;
    fx.eq_filter_res = 5.0;
    fx.firmware_fx = fw;
    fx
}

fn rms_db(fx: &mut LayerFxChain, cycles: f32) -> f32 {
    let n = 16_000;
    let (mut i2, mut o2) = (0.0f64, 0.0f64);
    for k in 0..n {
        let x = 0.1 * (std::f32::consts::TAU * cycles * k as f32).sin();
        let y = fx.process_frame([x, x])[0];
        if k >= n / 2 {
            i2 += f64::from(x * x);
            o2 += f64::from(y * y);
        }
    }
    (10.0 * (o2 / i2).log10()) as f32
}

#[test]
fn opt_in_off_by_default_and_not_in_all() {
    let (d, all) = (
        std::hint::black_box(FirmwareFx::DEFAULT),
        std::hint::black_box(FirmwareFx::ALL),
    );
    assert!(!d.lp24 && !all.lp24);
    let mut f = FirmwareFx::OFF;
    f.enable_ports("lp24").unwrap();
    assert!(f.lp24 && f.port("lp24"));
}

#[test]
fn lp24_runs_the_firmware_pair_with_four_frames_latency() {
    let mut fx = chain(AmpCabinet::LpFilter, LP24_ONLY);
    let first: Vec<f32> = (0..4).map(|_| fx.process_frame([1.0, 1.0])[0]).collect();
    assert!(first.iter().all(|v| v.abs() < 1e-6), "{first:?}");
    assert!(fx.fw_lp24.is_some());
}

#[test]
fn lp24_and_hp24_filter_in_the_right_direction() {
    let mut lp = chain(AmpCabinet::LpFilter, LP24_ONLY);
    let (lo, hi) = (rms_db(&mut lp, 0.002), rms_db(&mut lp, 0.3));
    assert!(lo > hi + 20.0, "LP24 {lo} vs {hi}");
    let mut hp = chain(AmpCabinet::HpFilter, LP24_ONLY);
    let (lo, hi) = (rms_db(&mut hp, 0.0005), rms_db(&mut hp, 0.3));
    assert!(hi > lo + 20.0, "HP24 {hi} vs {lo}");
}

#[test]
fn default_keeps_the_engine_path_and_other_amps_do_not_build_the_stage() {
    let mut fx = chain(AmpCabinet::LpFilter, FirmwareFx::OFF);
    let _ = fx.process_frame([1.0, 1.0]);
    assert!(fx.fw_lp24.is_none());
    let mut twin = chain(AmpCabinet::Twin, LP24_ONLY);
    let _ = twin.process_frame([1.0, 1.0]);
    assert!(twin.fw_lp24.is_none());
}

/// D234: the table repeats 1000 Hz at indices 65 and 66; the program's exact index must reach the
/// stage (a semitone of cutoff apart), and a stale index after a Hz edit is ignored.
#[test]
fn the_exact_mid_freq_index_reaches_the_stage() {
    let run = |hint: Option<u8>, hz: f32| {
        let mut fx = chain(AmpCabinet::LpFilter, LP24_ONLY);
        fx.eq_mid_freq_hz = hz;
        fx.eq_mid_freq_index = hint;
        rms_db(&mut fx, 0.02)
    };
    let (a, b) = (run(Some(65), 1000.0), run(Some(66), 1000.0));
    assert!((a - b).abs() > 0.05, "indices 65 / 66 sound alike: {a} {b}");
    // No hint (GUI) and a stale hint fall back to the nearest step.
    assert_eq!(run(None, 1000.0), run(Some(10), 1000.0));
    assert_eq!(run(None, 1000.0), run(Some(65), 1000.0));
}
