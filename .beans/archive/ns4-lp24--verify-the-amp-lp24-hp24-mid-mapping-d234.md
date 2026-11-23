---
# ns4-lp24
title: 'Verify the Amp LP24 / HP24 Mid mapping on the firmware and the oracle (D234)'
status: completed
type: task
created_at: 2026-11-23T02:30:00Z
updated_at: 2026-11-23T04:00:00Z
---

## Context
D193 / D223 kept the `lp24` FX port opt-in: the Mid knob to `kAmpMid` mapping, the Mid Freq index and the rate scale were ASSUMED.

## Higher Goal
1:1 audio parity with the firmware; a flip decision for `lp24` needs verified mappings.

## Acceptance Criteria
- [x] Firmware recall of the 26 LP24 / HP24 programs equals the decoder values (`tools/arm_program_amp_mid.py`)
- [x] Response per Mid position of the Rust stage against the oracle within 0.01 dB (54 settings)
- [x] Mapping defect fixed behind the opt-in (Hz-only Mid Freq index)
- [x] Tests, research note, D234, effects spec

## Out of Scope
- Flipping `lp24` (the owner decides; proposal in the research note); the default Amp EQ port index fix; piano timbre classes 2 / 3 (D209)

## Summary of Changes
See `docs/research/fx-lp24-mid-mapping.md` and D234.
