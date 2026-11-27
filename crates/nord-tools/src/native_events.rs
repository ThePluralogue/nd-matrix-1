//! Derived note / gate events of one synth layer, in the JSON shape of the ARM side
//! (`tools/arm_native_event_diff.py` reads both): `{voice_on_ms, voice_off_ms, gate_edges}`.
//! Used by the `native_event_trace` bin. Offline tooling only, no audio is stored.

use nord_core::synth::voice_edges::VoiceEdge;
use serde_json::{json, Value};

/// Level the Gate-mode float crosses to count as an edge.
/// ASSUMED: 0.5, the midpoint of the 0/1 float levels the ARM writes (Dual Gates: 1.0 / 0.0);
/// the native gate is a smoothed level, so any midpoint gives a crossing time within the ramp.
pub const GATE_THRESHOLD: f32 = 0.5;

/// Time of a frame in milliseconds.
pub fn frame_ms(frame: u64, fs: f32) -> f64 {
    frame as f64 * 1000.0 / f64::from(fs)
}

/// Reports a crossing of [`GATE_THRESHOLD`] between two calls of [`GateEdgeTracker::step`].
#[derive(Debug, Clone, Copy, Default)]
pub struct GateEdgeTracker {
    high: Option<bool>,
}

impl GateEdgeTracker {
    pub fn new() -> Self {
        Self::default()
    }

    /// `Some(1.0)` on a rising crossing, `Some(0.0)` on a falling one. The first call only
    /// records the state.
    pub fn step(&mut self, level: f32) -> Option<f32> {
        let high = level >= GATE_THRESHOLD;
        let edge = match self.high {
            Some(prev) if prev != high => Some(if high { 1.0 } else { 0.0 }),
            _ => None,
        };
        self.high = Some(high);
        edge
    }
}

/// Events of one layer collected over a run.
#[derive(Debug, Clone, Default)]
pub struct LayerEvents {
    pub voice_on_ms: Vec<f64>,
    pub voice_off_ms: Vec<f64>,
    pub gate_edges: Vec<(f64, f32)>,
}

impl LayerEvents {
    /// Sorts the voice edges into on / off times.
    pub fn add_voice_edges(&mut self, edges: &[VoiceEdge], fs: f32) {
        for e in edges {
            let t = frame_ms(e.frame, fs);
            if e.on {
                self.voice_on_ms.push(t);
            } else {
                self.voice_off_ms.push(t);
            }
        }
    }

    pub fn to_json(&self, arp_mode: &str) -> Value {
        let round = |t: f64| (t * 1000.0).round() / 1000.0;
        json!({
            "arp_mode": arp_mode,
            "voice_on_ms": self.voice_on_ms.iter().map(|&t| round(t)).collect::<Vec<_>>(),
            "voice_off_ms": self.voice_off_ms.iter().map(|&t| round(t)).collect::<Vec<_>>(),
            "gate_edges": self.gate_edges.iter()
                .map(|&(t, v)| json!({"t_ms": round(t), "value": v}))
                .collect::<Vec<_>>(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gate_tracker_reports_crossings_only() {
        let mut t = GateEdgeTracker::new();
        assert_eq!(t.step(1.0), None, "first call records the state");
        assert_eq!(t.step(0.7), None);
        assert_eq!(t.step(0.3), Some(0.0));
        assert_eq!(t.step(0.1), None);
        assert_eq!(t.step(0.9), Some(1.0));
    }

    #[test]
    fn frames_convert_to_ms() {
        assert!((frame_ms(48_000, 48_000.0) - 1000.0).abs() < 1e-9);
    }

    #[test]
    fn voice_edges_split_into_on_and_off_and_serialize() {
        let mk = |frame, on| VoiceEdge {
            frame,
            on,
            note: 60,
            velocity: 100,
        };
        let mut ev = LayerEvents::default();
        ev.add_voice_edges(&[mk(0, true), mk(33_600, false)], 48_000.0);
        ev.gate_edges.push((124.0, 1.0));
        let v = ev.to_json("Gate");
        assert_eq!(v["voice_on_ms"][0], 0.0);
        assert_eq!(v["voice_off_ms"][0], 700.0);
        assert_eq!(v["gate_edges"][0]["t_ms"], 124.0);
        assert_eq!(v["arp_mode"], "Gate");
    }
}
