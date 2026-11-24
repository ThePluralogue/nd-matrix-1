---
# ns4-lle7
title: 'Opt-in LLE backend in nd-matrix-1/nord-plugin and D2/D119 decision'
status: todo
type: task
priority: normal
created_at: 2026-11-23T21:40:00Z
updated_at: 2026-11-23T21:40:00Z
parent: ns4-lle1
blocked_by:
    - ns4-lle6
---

## Context
D2 currently designates the high-level Rust DSP engine as the primary engine and D119 keeps the SHARC system run as a proposed opt-in. Once `ns4-lle6` proves a playable real-time LLE backend, the project can expose it in `nd-matrix-1` and `nord-plugin` and decide whether LLE replaces the firmware-derived HLE paths — which would also allow deleting all Class B/C firmware-derived files (`ns4-u8o2`) in one stroke while keeping only clean-room `CBIN`/`.ns4p`/`.ns4b` tools and pure math fallback DSP.

## Higher Goal
Users can select the real-time LLE backend when they supply their own OS update file, and the architectural decision (opt-in vs. default replacement of firmware-derived HLE) is recorded in `docs/DECISIONS.md`.

## Acceptance Criteria
- [ ] Expose an opt-in `--engine lle` / Settings toggle in `nd-matrix-1` and `nord-plugin` when the user has configured their local OS update file
- [ ] Graceful fallback (or clear prompt to supply the OS update file) when no OS update file is configured
- [ ] Verify real-time stability in standalone and in DAW hosts (CLAP/VST3)
- [ ] Record the architectural decision updating D2/D119 in `docs/DECISIONS.md` and coordinate with `ns4-u8o2` / `ns4-eh57` on retiring firmware-derived HLE tables
- [ ] Unit tests for engine selection and missing-firmware fallback

## Out of Scope
Shipping any firmware or sample files; paid counsel review.
