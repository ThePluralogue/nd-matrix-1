---
# ns4-nev1
title: ARM vs native DSP event diff tooling
status: in-progress
type: task
priority: normal
created_at: 2026-11-23T20:00:00Z
updated_at: 2026-11-23T20:00:00Z
---

## Context
PR 633 found that Gate arp mode toggles float gate levels instead of retriggering voices. Preset sounds that differ from the hardware can be found systematically by diffing the firmware's derived DSP events against the native engine's, per factory program and synth layer.

## Higher Goal
Rank the mismatch classes by programs affected so each fix PR targets the biggest class first.

## Acceptance Criteria
- [x] `native_event_trace` bin emits `{program, layers: {A,B,C: {voice_on_ms, voice_off_ms, gate_edges}}}` for every factory program (keys 60,64,67 vel 100, hold 1.0 s, release 0.7 s, 120 BPM)
- [x] `tools/arm_native_event_diff.py` classifies voice retrigger count, voice count, step period, gate duty, key-off time, DSP words without native counterpart, with tests
- [ ] Ranked CSV and research note regenerated from the full ARM sweep (`docs/research/data/arm-dsp-events.json`, sibling branch `feat/arm-dsp-event-sweep`)

## Out of Scope
Any engine behaviour change.

## Summary of Changes
Native trace bin + `nord_tools::native_events`, `SynthLayer::gate_level()` accessor, diff script and tests, report built from the Dual Gates ARM numbers only until the full sweep lands.
