---
# ns4-bust
title: 'sharc-live bus tap: rank Simple Tune (layer B) and explain the silent ring tap'
status: completed
type: task
created_at: 2026-11-23T00:00:00Z
updated_at: 2026-11-23T00:00:00Z
parent: ns4-trkb
---

## Context
D230 traced Simple Tune (piano layer B, core 2) but could not rank it: its voices write bus `0x9405A`, which the post stage ring tap does not carry. D226 saw the ring tap silent after frame ~3,800 of a held-note run, so the release voice level in the tap was open.

## Higher Goal
1:1 audio parity with the firmware: a ranked chain tap for every piano program and a tap that reads the whole note and its release.

## Acceptance Criteria
- [x] `sharc-live` bus tap `B ADDR [i|f [PATH]]`, off by default, with unit tests (`live_bus`, `live_sy_tests`, `live_tests`)
- [x] Harness: `arm_dsp_live.py --bus-tap/--bus-side/--off-ticks`, `arm_smp_flash.attach(bus=, bus_tap=)`, `tools/smp_bus_rank.py` (+ tests)
- [x] Simple Tune ranked in `program-ranking.csv` (one row, CSV only): -27.0 dBFS held note, `smp_cycles` 0.01205, flag `piano_chain_no_audio` removed
- [x] Release tap question answered: the ring gate closes after ~2,300 frames, the bus keeps the note; Astoria Grand release +6.9 dB over the held note
- [x] Research note [smp-bus-tap](../../docs/research/smp-bus-tap.md), D233, audio-remaining row 24, OQ41
- [x] No default flips

## Out of Scope
Default flips, layer B of the 33 two-layer programs, identifying the post stage gate words, the dry native piano render, a census rerun.

## Summary of Changes
New `B` command in `sharc-live` (`runner/live_bus.rs`, `SyModel::bus_tap` and `bus_file`), harness flags in `arm_dsp_live.py`, `smp_bus_rank.py` for the CSV record, Simple Tune row, [smp-bus-tap](../../docs/research/smp-bus-tap.md) and D233.
