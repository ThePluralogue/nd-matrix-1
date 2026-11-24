# ARM vs native DSP event diff

Checked: 2026-11-23

Sources:
- ARM events: `docs/research/data/arm-dsp-events-dual-gates.json` (derived DSP events of the firmware).
- Native events: `native-dsp-events.json` (`native_event_trace` bin of `nord-tools`).
- Diff: `tools/arm_native_event_diff.py`; ranked rows in `docs/research/data/arm-native-event-diff.csv`.

Question: which factory programs make the firmware send different note / gate events than the native engine for the same keys (60, 64, 67, velocity 100, held 1.0 s, released at 0.7 s, 120 BPM)?

Programs compared: 1 (ARM programs without a native run: 0).

**Scope: partial.** The full ARM sweep (`docs/research/data/arm-dsp-events.json`, branch `feat/arm-dsp-event-sweep`) had not landed, so only Dual Gates layer A is compared, from the ARM numbers measured before the sweep (gate floats 1.0 at 124 ms, 0.0 at 208 ms, 1.0 at 249 ms at 120 BPM; chord key-on once at 0 ms, key-off once at 700 ms; VERIFIED by the ARM probe, one run). With one program every affected-program count is 0 or 1; layers the ARM file lacks are skipped. The native side covers all 384 factory programs (`docs/research/data/native-dsp-events.json`: 0 panics, 6 layers with Gate edges). Native Gate edges are crossings of 0.5 by the smoothed gate level (ASSUMED threshold). Regenerate with `python tools/arm_native_event_diff.py --native docs/research/data/native-dsp-events.json --md docs/research/arm-native-event-diff.md --checked <date>`.

## Classes by programs affected

| Rank | Class | Programs | Layers | Tag |
|---|---|---|---|---|
| 1 | `gate_duty` | 1 | 1 | EMPIRICAL (0.10 tolerance ASSUMED) |
| 2 | `voice_retrigger` | 0 | 0 | VERIFIED (event counts) |
| 3 | `voice_count` | 0 | 0 | VERIFIED (event counts) |
| 4 | `step_period` | 0 | 0 | EMPIRICAL (5 % tolerance ASSUMED) |
| 5 | `key_off` | 0 | 0 | EMPIRICAL (25 ms tolerance ASSUMED) |
| 6 | `dsp_word_missing` | 0 | 0 | VERIFIED (word list) |

## Largest mismatch per class

| Class | Program | Layer | ARM | Native | Detail |
|---|---|---|---|---|---|
| `gate_duty` | Dual Gates.ns4p | A | 0.672 | 0.500 | high fraction of a period |

## Implications

- Classes at the top affect the most programs; fix them first, one PR per class.
- Tolerances are tooling choices (`ASSUMED`); counts are exact.
- Rows come from derived events only, so a class can hide another cause (for example a `voice_count` hit from a key split).

## Follow-ups

- Re-run on the full ARM sweep (`docs/research/data/arm-dsp-events.json`) when it lands.
- Add a native counterpart per `dsp_word_missing` word, or document why none is needed.
