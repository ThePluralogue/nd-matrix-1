//! Native counterpart of the ARM DSP event sweep: for every factory program of a backup (`.ns4b`)
//! holds keys 60,64,67 (velocity 100) for 1.0 s, releases at 0.7 s at tempo 120 and prints the
//! derived events per synth layer A/B/C (voice note-on / note-off times, Gate-mode float edges).
//! `tools/arm_native_event_diff.py` sets the output against `docs/research/data/arm-dsp-events.json`.
//!
//! Usage: `cargo run --release -p nord-tools --bin native_event_trace -- <backup.ns4b>
//! [--limit N] [--only S] [--skip N] [--every K] [--keys 60,64,67] [--velocity V] [--tempo T]
//! [--hold S] [--release-at S] [--out file.json]`. Output holds derived numbers only.

use nord_core::cbin::{CbinFile, Ns4BackupArchive};
use nord_core::engine::Stage4Engine;
use nord_core::midi::MidiEvent;
use nord_core::render_opts::FirmwareOptions;
use nord_core::synth::voice_edges::{VoiceEdge, VoiceEdgeTracker};
use nord_core::synth::SynthLayer;
use nord_tools::native_events::{frame_ms, GateEdgeTracker, LayerEvents};
use nord_tools::sweep_load::{
    build_engine, flag, load_sounds, panic_message, PianoCache, SynthCache, FS,
};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs::File;
use std::panic::{catch_unwind, AssertUnwindSafe};

struct Plan {
    keys: Vec<u8>,
    velocity: u8,
    tempo: f32,
    hold_s: f64,
    release_s: f64,
}

fn parse_keys(text: &str) -> Result<Vec<u8>, String> {
    text.split(',')
        .map(|k| k.trim().parse::<u8>().map_err(|_| format!("bad key {k:?}")))
        .collect()
}

fn layers(e: &Stage4Engine) -> [&SynthLayer; 3] {
    [&e.synth_a, &e.synth_b, &e.synth_c]
}

/// Plays the plan and collects the events of the three synth layers.
fn trace(engine: &mut Stage4Engine, plan: &Plan) -> Vec<(String, LayerEvents)> {
    engine.master_clock.bpm = plan.tempo;
    let mut voice = [
        VoiceEdgeTracker::new(),
        VoiceEdgeTracker::new(),
        VoiceEdgeTracker::new(),
    ];
    let mut raw: [Vec<VoiceEdge>; 3] = [Vec::new(), Vec::new(), Vec::new()];
    let mut gate = [GateEdgeTracker::new(); 3];
    let mut out: Vec<LayerEvents> = vec![LayerEvents::default(); 3];
    let release_frame = (plan.release_s * f64::from(FS)) as u64;
    let end = (plan.hold_s * f64::from(FS)) as u64;
    for &note in &plan.keys {
        engine.handle_midi(MidiEvent::NoteOn {
            channel: 0,
            note,
            velocity: plan.velocity,
        });
    }
    for frame in 0..end {
        if frame == release_frame {
            for &note in &plan.keys {
                engine.handle_midi(MidiEvent::NoteOff {
                    channel: 0,
                    note,
                    velocity: 0,
                });
            }
        }
        engine.render_frame();
        for (i, layer) in layers(engine).into_iter().enumerate() {
            voice[i].step(layer, frame, &mut raw[i]);
            if let Some(v) = gate[i].step(layer.gate_level()) {
                out[i].gate_edges.push((frame_ms(frame, FS), v));
            }
        }
    }
    for (ev, edges) in out.iter_mut().zip(&raw) {
        ev.add_voice_edges(edges, FS);
    }
    ["A", "B", "C"]
        .iter()
        .map(|s| s.to_string())
        .zip(out)
        .collect()
}

fn program_json(name: &str, engine: &Stage4Engine, events: Vec<(String, LayerEvents)>) -> Value {
    let mut map = serde_json::Map::new();
    for ((k, ev), layer) in events.into_iter().zip(layers(engine)) {
        map.insert(k, ev.to_json(&format!("{:?}", layer.arp_mode)));
    }
    json!({"program": name, "layers": map})
}

fn num<T: std::str::FromStr>(args: &[String], name: &str, d: T) -> Result<T, String> {
    flag(args, name).map_or(Ok(d), |v| v.parse().map_err(|_| format!("bad {name}")))
}

fn run(args: &[String]) -> Result<(), String> {
    let path = args
        .get(1)
        .ok_or("usage: native_event_trace <backup.ns4b>")?;
    let plan = Plan {
        keys: parse_keys(flag(args, "--keys").map_or("60,64,67", String::as_str))?,
        velocity: num(args, "--velocity", 100u8)?,
        tempo: num(args, "--tempo", 120.0f32)?,
        hold_s: num(args, "--hold", 1.0f64)?,
        release_s: num(args, "--release-at", 0.7f64)?,
    };
    let limit: usize = num(args, "--limit", usize::MAX)?;
    let every: usize = num(args, "--every", 1usize)?.max(1);
    let skip: usize = num(args, "--skip", 0usize)?;
    let only = flag(args, "--only");
    let archive = Ns4BackupArchive::open(path).map_err(|e| e.to_string())?;
    let mut file = File::open(path).map_err(|e| e.to_string())?;
    let (mut pianos, mut synths): (PianoCache, SynthCache) = (HashMap::new(), HashMap::new());
    let names: Vec<String> = archive
        .entries_with_ext(".ns4p")
        .iter()
        .map(|e| e.name.clone())
        .filter(|n| only.is_none_or(|s| n.contains(s.as_str())))
        .skip(skip)
        .step_by(every)
        .take(limit)
        .collect();
    std::panic::set_hook(Box::new(|_| {}));
    let opts = FirmwareOptions::default();
    let mut programs = Vec::new();
    for name in names {
        let bytes = archive
            .read_entry(&mut file, &name)
            .map_err(|e| e.to_string())?;
        let prog = CbinFile::parse(&bytes)
            .and_then(|c| c.decode_program())
            .map_err(|e| format!("{name}: {e}"))?;
        let refs = archive.resolve_program_sounds(&prog);
        load_sounds(&archive, &mut file, &refs, &mut pianos, &mut synths);
        let short = name.rsplit(['/', '\\']).next().unwrap_or(&name).to_string();
        let r = catch_unwind(AssertUnwindSafe(|| {
            let mut engine = build_engine(&prog, &refs, &pianos, &synths, &opts);
            let ev = trace(&mut engine, &plan);
            program_json(&short, &engine, ev)
        }));
        programs.push(r.unwrap_or_else(|p| json!({"program": short, "panic": panic_message(p)})));
    }
    let doc = json!({
        "source": "native",
        "keys": plan.keys, "velocity": plan.velocity, "tempo_bpm": plan.tempo,
        "release_ms": plan.release_s * 1000.0, "hold_ms": plan.hold_s * 1000.0,
        "programs": programs,
    });
    let text = serde_json::to_string(&doc).map_err(|e| e.to_string())?;
    match flag(args, "--out") {
        Some(p) => std::fs::write(p, text).map_err(|e| format!("{p}: {e}")),
        None => {
            println!("{text}");
            Ok(())
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if let Err(e) = run(&args) {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_parse_and_reject_garbage() {
        assert_eq!(parse_keys("60, 64,67").unwrap(), vec![60, 64, 67]);
        assert!(parse_keys("60,x").is_err());
    }

    #[test]
    fn plain_engine_reports_three_layers_with_no_gate_edges() {
        let mut engine = Stage4Engine::new(FS);
        let plan = Plan {
            keys: vec![60, 64, 67],
            velocity: 100,
            tempo: 120.0,
            hold_s: 1.0,
            release_s: 0.7,
        };
        let ev = trace(&mut engine, &plan);
        assert_eq!(ev.len(), 3);
        assert_eq!(ev[0].0, "A");
        let json = program_json("x.ns4p", &engine, ev);
        assert_eq!(json["layers"]["A"]["arp_mode"], "Off");
        assert!(json["layers"]["B"]["gate_edges"]
            .as_array()
            .unwrap()
            .is_empty());
    }
}
