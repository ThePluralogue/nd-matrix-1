---
# ns4-lle1
title: 'Phase 8: Full Real-Time ARM + SHARC Emulation (LLE)'
status: todo
type: epic
priority: normal
created_at: 2026-11-23T21:40:00Z
updated_at: 2026-11-23T21:40:00Z
---

## Context
Track B (`ns4-trkb`, `crates/nord-sharc`) already boots all five DSP images (`ORG`, `PNO`, `FX`, `SY1SY2`, `SMP`), matches the interpreter 316/316 passes in Cranelift JIT on PNO core 1 (`docs/research/pno-jit-arm-profile.md`), and runs 152/155 piano programs through `SMP` CNSP page refill (`ns4-pnoc`), while the ARM0 firmware boots in Unicorn (`tools/arm_*.py`).

Today Track B is an offline test oracle (`docs/specs/sharc-core.md`, D2, D119). Promoting it to a **full real-time ARM + SHARC low-level emulator (LLE)** offers two major benefits:
1. **Bit-exact audio, FX, arpeggiator, and OLED fidelity** across all programs and future OS updates without hand-porting every DSP routine.
2. **Cleanest copyright posture:** pure hardware emulation (Analog Devices SHARC + ARM) contains zero vendor DSP logic or lookup tables; users supply their own lawfully obtained, unencrypted OS update file.

This epic gates the full wiring behind a **Gate 1 feasibility & speed benchmark** (`ns4-lle2`, `ns4-lle3`) to prove real-time 48 kHz viability before committing to full GUI/plugin integration.

## Higher Goal
Run the complete Nord Stage 4 hardware architecture (ARM0 + all 5 SHARC DSP images) in real time inside a single Rust engine from the user's own OS update and `.ns4b` backup, with zero vendor code or tables in the repository.

## Acceptance Criteria
Gate 1 — Real-time feasibility & speed:
- [ ] `ns4-lle2` Rust in-process ARM0 emulator with idle-skipping (replaces Python/Unicorn harness; hits < 1 ms per 1 ms tick)
- [ ] `ns4-lle3` Multi-core real-time SHARC JIT/AOT benchmark at 48 kHz (PNO + ORG + FX + SY1SY2 + SMP in parallel within a 5–10 ms audio buffer)

Gate 2 — Unified multi-chip audio & memory:
- [ ] `ns4-lle4` Live chip-to-chip `SPORT`/audio DMA bus (`ORG`/`PNO`/`SY`/`SMP` → `FX Core1/Core2` → stereo out), FX Core 2 poll-loop fix, and external SDRAM (`0x20xxxxxx`) delay/reverb window
- [ ] `ns4-lle5` Unified Rust multi-chip LLE engine orchestrating ARM0 + all 5 DSP images + DSP→ARM return path (`ns4-nds8`) + `SMP` CNSP page refill (`ns4-pnoc`)

Gate 3 — Playable instrument & architectural decision:
- [ ] `ns4-lle6` Wire live MIDI, pedals, panel controls, `.ns4b` program store (`ns4-pyqg`), and real OLED framebuffer (`ns4-2gdc`) into the LLE engine
- [ ] `ns4-lle7` Expose opt-in LLE audio backend in `nd-matrix-1` / `nord-plugin` and record the D2/D119 architectural decision (whether LLE stays opt-in or replaces firmware-derived HLE paths, allowing deletion of all Class B/C files)

## Out of Scope
Shipping vendor firmware, samples, or presets; bypassing any encryption or DRM; removing the clean-room `CBIN`/`.ns4p`/`.ns4b` file-format toolkit.
