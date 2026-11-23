# Amp LP24 / HP24: the Mid knob mapping on the firmware and the oracle (D234)

- **Checked:** 2026-11-23 (`main` @ `2b8560b7`)
- **Sources (local, never committed):** `target/fw_unpacked/bank0_CODE.bin` and `bank0_SRAM.bin` (ARM OS v1.66), `target/oled/eeprom.bin`, the factory restore `.ns4b` under `.reference/`, the FX core 2 image `core2_sram.bin` (`sharc-fx`), `target/release/ns4`. Committed derived data: [fx-lp24-recall.csv](data/fx-lp24-recall.csv), [fx-lp24-mid-sweep.csv](data/fx-lp24-mid-sweep.csv), [fx-lp24-words.csv](data/fx-lp24-words.csv).
- **Question:** D193 and the default flip audit ([default-flip-audit](default-flip-audit.md), D223) keep `--firmware-fx-ports lp24` opt-in because the Mid knob to `kAmpMid` mapping, the Mid Freq to index mapping and the rate scale are ASSUMED. Do the two mappings hold on the firmware, and does the Rust stage match the oracle at every Mid position, not only at the six golden scenarios?

Tags follow `.agents/rules/provenance-in-code.md`.

## 1. Two separate claims

| Claim | Where it lives | How it is checked |
|---|---|---|
| A. file fields (Mid count, Mid Freq index) are the values the firmware feeds the routine | decoder (`cbin/fx_preset.rs`), program load (`engine/program_fx.rs`), `LayerFxChain::process_fw_lp24` | the booted ARM recalls the program itself; its `kAmpMid` / `kAmpMidFreq` cells are compared with the decoder (s2) |
| B. the Rust stage gives the oracle's response at every Mid position | `fx/fw_lp24.rs` | the oracle (`sharc-fx`, FX core 2 copy `0x124586`) runs the real template at 54 settings; the port is compared band by band (s3) |

The words behind a given index were already VERIFIED (D181, D193: all 128 + 121 words, both types). What was ASSUMED is the chain from the file to that index.

## 2. A: firmware recall of the 26 factory programs with an LP24 / HP24 amp

`tools/arm_program_amp_mid.py` puts a factory `.ns4p` into the Store record of the EEPROM image (`arm_program_load.build_image`), boots the full ARM, lets the firmware recall the program, and reads the value cells of `Edt_Effects_kAmpTypeNew` (`0x1A7`), `kAmpMid` (`0x1A0`) and `kAmpMidFreq` (`0x1A2`) on all 12 FX units. The expected values come from `ns4 inspect --json` (`amp_type_new & 15`, `eq_mid_db / 0.25`, `eq_mid_freq_norm * 127`).

Result ([fx-lp24-recall.csv](data/fx-lp24-recall.csv)): **26 of 26 programs, 35 distinct (type, Mid, Freq) tuples, every file tuple is recalled unchanged** (type, signed Mid count, Mid Freq index). Example: Unclocked Pad `(9, -60, 39)`, `(9, -60, 80)`, `(9, -60, 38)`; Sidechain Whl II `(8, -24, 85)`, `(9, -46, 55)`; FM Cloud Whl `(8, 18, 75)`, `(8, 24, 103)`. The Mid field of a LP24 / HP24 amp is the same signed count as for the other amps (`kAmpMid` -60..60).

Engine side: `FxDecoded::amp_filter_res` is the exact inverse of the count (`10 (60 + count) / 120`); `mid_counts_of_res` undoes it. `tests_fw_lp24_mapping.rs::every_mid_count_round_trips_through_the_resonance_knob` runs all 121 counts (error under 1e-3 counts).

### Defect found: the Mid Freq index was lost through Hz

The engine keeps the Mid Freq as Hz (`EQ_FREQ_HZ`, the display table) and the stage took the **nearest table step** of that Hz. The display table repeats values (`[65] = [66] = 1000`, `[67] = [68] = 1100`, and so on through 2400 Hz), so Hz alone cannot name the index. Over the 128 indices, `freq_index_of_hz(EQ_FREQ_HZ[i]) != i` for 12 of them (66, 68, 70, 72, 74, 76, 78, 80, 82, 84, 87, 91; the nearest step breaks a tie to the lower index; test `every_mid_freq_index_reaches_the_stage_exactly` asserts at least 10). One index is one semitone of cutoff (the cutoff words double every 12 steps), so the old mapping put the cutoff up to one semitone off whenever the file index sat on a repeated step. In the 26 factory programs this hits indices `76`, `80`, `84` and `87`: 6 programs (Crackle n Pd Whl 76 and 80, Stereo AnaStr II 80, Lost Cause EP 80, PineScape Synth 87, Singing Upright 84, Unclocked Pad 80; the `lossy_hz_indices` column).

Fix, behind the existing opt-in only: `LayerFxChain::eq_mid_freq_index: Option<u8>` is set by the program load (`FxDecoded::eq_mid_freq_index`) and by the Mid Freq morph, copied by `sharing.rs`; `fw_eq::freq_index(hz, hint)` uses it while `EQ_FREQ_HZ[hint] == eq_mid_freq_hz` (a GUI edit of the Hz invalidates it, so a stale hint is ignored) and otherwise keeps the nearest step. Only `process_fw_lp24` uses it; the default render and `render_hash` do not change (the field is `None` unless a program sets it, and only `lp24` reads it).

## 3. B: response per Mid position against the oracle

`tools/fx_lp24_mid_sweep.py run` drives the oracle with the real template and ARM words, **settled** smoothers (steady response), 128 calls of seeded noise (512 stereo frames, amplitude 0.4, seed 7000) for LP24 and HP24 at `kAmpMid` -60 .. 60 in steps of 15 and `kAmpMidFreq` 60 / 90 / 110 (54 scenarios). The output power gain of the left channel is summed in 12 DFT bands ([fx-lp24-mid-sweep.csv](data/fx-lp24-mid-sweep.csv)). `tests_fw_lp24_sweep.rs` runs the Rust stage on the same input and compares the 12 bands.

Table: peak band gain of the oracle (dB) / worst band difference Rust against oracle (dB), from `cargo test -p nord-core --lib fx::tests_fw_lp24_sweep -- --nocapture`:

| kAmpMid | LP24 @60 | LP24 @90 | LP24 @110 | HP24 @60 | HP24 @90 | HP24 @110 |
|---|---|---|---|---|---|---|
| -60 | -0.0 / 0.0001 | +0.6 / 0.0001 | +0.7 / 0.0000 | -0.5 / 0.0001 | -2.6 / 0.0001 | -6.1 / 0.0001 |
| -45 | +0.1 / 0.0001 | +0.8 / 0.0000 | +0.9 / 0.0001 | +0.3 / 0.0001 | -1.5 / 0.0001 | -4.5 / 0.0001 |
| -30 | +0.5 / 0.0001 | +1.2 / 0.0001 | +1.1 / 0.0000 | +2.5 / 0.0001 | +1.1 / 0.0001 | -1.6 / 0.0001 |
| -15 | +3.1 / 0.0001 | +3.2 / 0.0001 | +3.1 / 0.0000 | +4.5 / 0.0000 | +3.8 / 0.0001 | +0.9 / 0.0002 |
| 0 | +5.2 / 0.0001 | +6.2 / 0.0001 | +5.5 / 0.0001 | +6.2 / 0.0001 | +6.1 / 0.0001 | +3.3 / 0.0003 |
| 15 | +4.9 / 0.0001 | +7.8 / 0.0001 | +7.0 / 0.0000 | +8.4 / 0.0001 | +9.0 / 0.0001 | +7.8 / 0.0001 |
| 30 | +4.4 / 0.0001 | +10.0 / 0.0001 | +9.0 / 0.0001 | +10.9 / 0.0001 | +14.8 / 0.0001 | +13.6 / 0.0001 |
| 45 | +5.4 / 0.0001 | +13.6 / 0.0000 | +12.2 / 0.0001 | +15.6 / 0.0001 | +20.6 / 0.0001 | +18.6 / 0.0001 |
| 60 | -6.4 / 0.0018 | +17.6 / 0.0008 | +16.7 / 0.0002 | +28.9 / 0.0001 | +25.6 / 0.0002 | +22.4 / 0.0001 |

Worst difference over the 54 settings and 12 bands: **0.0018 dB** (LP24 at index 60, Mid 60, a band 6 dB below the input), against a tolerance of 0.01 dB. The Python model against the oracle words (`word_err` column): at most `2.1e-5` on outputs whose peak band is up to +29 dB (clipped states, `[-1, 1]`), i.e. under `1e-6` of the output peak. The response spans -6 .. +29 dB over the knob, so the sweep is not flat. The Mid knob raises the resonance and the pre-gain, as a resonance control: at Mid 0 the peak is +3 .. +6 dB, at Mid 60 up to +29 dB (HP24).

## 4. Tags after this note

| Item | Before | After |
|---|---|---|
| Mid count to `kAmpMid` (file, decoder, engine round trip) | ASSUMED | VERIFIED (26 of 26 recalls, 121 of 121 round trip) |
| Mid Freq to `kAmpMidFreq` index | ASSUMED (and wrong on 12 of 128 indices) | VERIFIED after the fix (26 of 26 recalls, 128 of 128 indices reach the stage) |
| Response per Mid position, stage against oracle | one golden per scenario | VERIFIED, 54 settings, 0.0018 dB worst |
| Rate scale (the recurrence runs at the engine rate, the firmware at 43,402.34 Hz) | ASSUMED | ASSUMED (unchanged; the FX rate match of D180 applies through the same path as for the drive stage) |
| Hardware clip of an LP24 / HP24 program | none | none |

## Flip proposal (not made in this PR)

Per D223 point (2): with no hardware clip the effect on parity cannot be measured and the owner's explicit OK is still required. What is left for `lp24` is the rate scale and the absence of a clip. The mapping side of the audit row is closed. **Proposal:** add `lp24` to `PARITY`, in one PR with the `render_hash` and golden re-baseline (D22 criterion 3), when the owner approves; the programs that change are the 26 of [program-ranking-amp-lp24.csv](data/program-ranking-amp-lp24.csv) (mean distance to the all-firmware render 5.29 to 1.59, 13 better, 13 equal, 0 worse, D193). A clip of Unclocked Pad (F:46) or Dance Lead (C:54), both HP24, would settle the rate scale and the cutoff unit.

## Not done

- The EMPIRICAL piano timbre classes 2 / 3 (D209): the class field of those programs is not decoded, and the ARM oracle of D209 forces the timbre mode rather than recalling it, so the check is not a rerun of this method. Not attempted here; see Follow-ups.

## Implications

- `lp24` mapping tags move from ASSUMED to VERIFIED; the flip decision is the owner's.
- The same Hz-only index loss exists in the default Amp EQ port (`process_fw_eq` calls `freq_index_of_hz(eq_mid_freq_hz)`): the mid band centre is up to one index step off for the 12 lossy indices when the Mid band gain is not neutral. That path is on by default, so fixing it changes the default render and needs its own `render_hash` re-baseline.

## Follow-ups

1. Use `fw_eq::freq_index(hz, hint)` in `process_fw_eq` (default `eq` port, D170): one-line change, re-baseline `render_hash` and the goldens, check the clips (separate PR).
2. Piano timbre classes 2 / 3 (D209): decode the class field of the piano header, then recall the factory programs of those classes in the booted firmware and compare the stored mode with the `kEq % 6` map.
3. A hardware clip of an LP24 / HP24 program (rate scale, cutoff unit).
