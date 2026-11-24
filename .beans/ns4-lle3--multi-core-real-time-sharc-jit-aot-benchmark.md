---
# ns4-lle3
title: 'Multi-core real-time SHARC JIT/AOT benchmark at 48 kHz'
status: todo
type: task
priority: high
created_at: 2026-11-23T21:40:00Z
updated_at: 2026-11-23T21:40:00Z
parent: ns4-lle1
---

## Context
`crates/nord-sharc` already has a Cranelift JIT that matches the interpreter 316/316 passes on PNO core 1 (`sharc-sys --jit-diff --strict`) as well as fast-float (`f32`) JIT and AOT prototypes. Before wiring a full multi-chip runtime, we must prove whether all active SHARC cores (`ORG c1/c2`, `PNO c1/c2`, `FX c1/c2`, `SY1SY2`, `SMP`) can meet a 48 kHz real-time audio buffer budget (e.g., 128–512 frames = 2.7–10.7 ms) on consumer laptop CPUs.

## Higher Goal
Establish a hard go/no-go real-time CPU budget for running all SHARC cores concurrently at 48 kHz using the Cranelift JIT or AOT backend.

## Acceptance Criteria
- [ ] Build a multi-threaded benchmark binary in `crates/nord-tools` (or `crates/nord-sharc`) that executes representative passes across `PNO`, `ORG`, `FX`, `SY1SY2`, and `SMP` cores concurrently using the JIT/AOT engine
- [ ] Measure per-core pass latency, p99 jitter, and total CPU usage at 48 kHz (128, 256, and 512 frame buffers) in both exact-40-bit and fast-`f32` modes
- [ ] Identify top hot spots if any core misses real-time (e.g., Cranelift lowering gaps, lock contention, cache misses) and record results in `docs/research/`
- [ ] Unit tests verifying the benchmark harness on synthetic SHARC programs

## Out of Scope
Full ARM0 integration (`ns4-lle5`); SPORT audio routing (`ns4-lle4`).
