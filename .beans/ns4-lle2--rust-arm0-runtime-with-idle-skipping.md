---
# ns4-lle2
title: 'Rust ARM0 runtime with idle-skipping (replace Python/Unicorn harness)'
status: completed
type: task
priority: high
created_at: 2026-11-23T21:40:00Z
updated_at: 2026-11-23T23:30:00Z
parent: ns4-lle1
---

## Context
`docs/research/pno-jit-arm-profile.md` measured the Python/Unicorn ARM0 harness at 18–24 ms per 1 ms tick (8–11 ms even with all 34 Python hooks removed) against an ASSUMED 200,000 instructions/ms budget. Real-time LLE requires ARM0 to run inside Rust with zero Python/IPC overhead and with idle/poll-loop skipping.

## Higher Goal
ARM0 boots the user's OS update image in Rust and advances in real time (< 1 ms wall-clock per 1 ms virtual time) while emitting live ARM→DSP messages and accepting DSP→ARM reads.

## Acceptance Criteria
- [x] Embed ARM0 execution in Rust (e.g., `unicorn-engine` Rust bindings or native Rust ARMv7/Thumb core) behind an opt-in feature/crate
- [x] Port the essential hardware stubs from `tools/arm_org_stream.py` / `tools/arm_dsp_return*.py` (timer IRQ, DSP mailboxes, SPI peek return path) to Rust with zero Python callbacks
- [x] Implement idle / `WFI` / poll-loop fast-forwarding so ARM0 does not burn full 200k-instruction ticks while waiting for the next timer interrupt
- [x] Benchmark wall-clock cost per 1 ms tick on a real OS v1.66 boot + program load + key events; verify < 1.0 ms/tick (real-time headroom) and record measurements in `docs/research/`
- [x] Unit tests with synthetic ARM code fixtures in memory (no firmware bytes committed)

## Out of Scope
Wiring live GUI controls (`ns4-lle6`); multi-chip SHARC audio routing (`ns4-lle4`).

## Summary of Changes
- New crate `crates/nord-arm`: native ARM926EJ-S interpreter (ARM + Thumb), sparse memory, boot hooks, tick/GPIO/poll devices, panel, I2C EEPROMs, UART2 MIDI, DSP port log, DSP to ARM SPI return, exact idle/WFI skipping. Decision D244 (Unicorn rejected: GPLv2, callbacks).
- `arm-lle-bench` (`nord-tools`): OS v1.66 boot + program load + key events. Mean 0.21 to 0.41 ms/tick, p99 under 0.9 ms for key phases; 4.8 % of load ticks and 2.5 % of eight-key ticks exceed 1 ms (worst 5.7 ms). Skipping state hash equals full execution after every phase.
- Docs: `docs/research/arm0-rust-runtime.md`, `docs/specs/arm0-runtime.md`, README index, ROADMAP Gate 1a.
- Follow-ups: predecode cache for about 200 MIPS, measure real instructions per tick, null-call 747 vs 762 difference, live DSP return layers (`ns4-lle5`).
