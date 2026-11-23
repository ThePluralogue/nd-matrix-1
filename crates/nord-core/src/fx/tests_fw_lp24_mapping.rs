//! Knob mapping of the firmware Amp LP24 / HP24 port (D234): the engine fields the stage reads
//! (`eq_filter_res`, `eq_mid_freq_hz`) round trip to the file's Mid count and Mid Freq index, which
//! the booted firmware recalls unchanged (`tools/arm_program_amp_mid.py`, `fx-lp24-recall.csv`).

use super::fw_eq;
use super::fw_lp24::mid_counts_of_res;
use super::{eq_freq_hz_from_norm, EQ_GAIN_DB_PER_COUNT};
use crate::cbin::fx_preset::FxDecoded;

/// Every signed 7-bit Mid count the firmware accepts (`kAmpMid` -60..=60) survives the file value
/// -> `eq_mid_db` -> resonance knob -> counts path exactly.
#[test]
fn every_mid_count_round_trips_through_the_resonance_knob() {
    for count in -60i32..=60 {
        let mut fx = FxDecoded::default();
        fx.eq_mid_db = count as f32 * EQ_GAIN_DB_PER_COUNT;
        let back = mid_counts_of_res(fx.amp_filter_res());
        assert!(
            (back - count as f32).abs() < 1.0e-3 && back.round() as i32 == count,
            "count {count} came back as {back}"
        );
    }
}

/// Every Mid Freq index `0..=127` of the file reaches the stage unchanged: through the exact index
/// the program load keeps next to the Hz. Hz alone repeats display steps (65 / 66, 67 / 68, ...).
#[test]
fn every_mid_freq_index_reaches_the_stage_exactly() {
    let mut repeated = 0;
    for index in 0..=127u8 {
        let hz = eq_freq_hz_from_norm(f32::from(index) / 127.0);
        assert_eq!(fw_eq::freq_index(hz, Some(index)), f32::from(index));
        repeated += usize::from(fw_eq::freq_index_of_hz(hz) != f32::from(index));
    }
    assert!(
        repeated >= 10,
        "the Hz table repeats steps: {repeated} lossy indices"
    );
}

#[test]
fn a_stale_hint_is_ignored_when_the_hz_changed() {
    // GUI edit after a program load: Hz moved, the hint no longer names it.
    assert_eq!(
        fw_eq::freq_index(2000.0, Some(10)),
        fw_eq::freq_index_of_hz(2000.0)
    );
    assert_eq!(
        fw_eq::freq_index(1000.0, None),
        fw_eq::freq_index_of_hz(1000.0)
    );
    assert_eq!(
        fw_eq::freq_index(1000.0, Some(200)),
        fw_eq::freq_index_of_hz(1000.0)
    );
}

#[test]
fn the_program_load_keeps_the_exact_index() {
    let mut fx = FxDecoded::default();
    for index in [0u8, 39, 65, 66, 80, 127] {
        fx.eq_mid_freq_norm = f32::from(index) / 127.0;
        assert_eq!(fx.eq_mid_freq_index(), index);
        assert_eq!(
            fx.eq_mid_freq_hz(),
            eq_freq_hz_from_norm(f32::from(index) / 127.0)
        );
    }
}

#[test]
fn resonance_knob_ends_map_to_the_count_ends() {
    assert_eq!(mid_counts_of_res(0.0), -60.0);
    assert_eq!(mid_counts_of_res(5.0), 0.0);
    assert_eq!(mid_counts_of_res(10.0), 60.0);
    assert_eq!(mid_counts_of_res(99.0), 60.0);
}
