# Envelope pass period and the settled tables on the hardware clips

- **Checked:** 2026-11-23 (`main` @ `b6e562cc`, merged with `origin/main` before the PR)
- **Sources (local, never committed):** Part A video clips and the factory backup under `.reference/` (`tools/emu_vs_hw.py`), `target/release/{ns4,synth_env_native}`; committed: `crates/nord-core/src/cbin/presets.rs` (firmware display strings `0x53abe4` / `0x53ade4`), [synth-env-rates-settled.csv](data/synth-env-rates-settled.csv) (oracle installer words, D212), [synth-env-rate-table.csv](data/synth-env-rate-table.csv) (the stale sweep), [dsp-frame-rate](dsp-frame-rate.md) (`F`, D211), [synth-e2e-oracle-vs-native](synth-e2e-oracle-vs-native.md), [default-flip-audit](default-flip-audit.md) (D223).
- **Question:** D223 kept the settled envelope tables opt-in because 7 of 17 hardware clip segments scored worse (`synth_12db` 1.38 to 3.15) and "the pass period behind the tables is unconfirmed". Is the pass period (2 DSP frames) right with the settled tables, and why do the clips get worse?

Tags follow `.agents/rules/provenance-in-code.md`.

## 1. Answer

| Item | Result |
|---|---|
| Pass period | **2.000 DSP frames, VERIFIED on all 128 knobs** (attack median 1.9995, decay median 2.0007; section 2). With the stale sweep no single period fits (median 2.36 / 2.30, up to 5.2). The earlier check only used the plateau starts, where both tables agree, so it could not tell the tables apart. |
| Tables | The settled tables stay as measured (D212); nothing in them needs correcting. The one defect found: the panel time to index inverse (`attack_index`, `decay_index`) still searched the stale table, so a layer without stored knobs landed on a plateau start. Fixed behind the opt-in (`attack_index_with`, section 4). |
| Clips | Settled tables alone: mean 2.668 to 2.821, **8 of 17 segments worse** (7 distinct programs), 4 better, 5 equal. No combination with the other envelope opt-ins is better than the default (best mean 3.018). |
| Cause | Not the tables and not the period. The scored quantity is the 5 band spectral gap; the correct, slower envelope makes the emulator **darker** in presence and brilliance where it is already darker than the hardware (section 3). The stale tables were opening the filter up to 2.56 times too early, which hid a missing brightness in the default envelope to cutoff path (`2^(8 amount level)`, D200 / D207; ASSUMED level link of the firmware law, D223). |
| Flip | **No.** Not every segment is equal or better (section 5). Re-score after the cutoff law is fixed. |

## 2. Pass period (VERIFIED)

A pass is the control step of the amp, filter and osc ADR chains: one record step per pass, coefficients rewritten every 16th pass (D200 / D212; the oracle taps every pass). Its length `P` in DSP frames follows from two firmware tables, no ARM run needed:

| Quantity | Passes | Display |
|---|---|---|
| Attack | linear ramp to full scale: `1 / b` (settled attack coefficient of index `knob - 64`) | `ENV_ATTACK_MS_TABLE[knob]`, the firmware strings at `0x53abe4` (VERIFIED, `tools/ns4_fw_tables.py`) |
| Decay / release | fall by 40 dB: `ln(100) / (1 - a)` (one pole `y <- a y`) | `ENV_DECAY_MS_TABLE[knob]`, `0x53ade4` |

`P = display_s * F / passes`, `F = 44,444,000 / 1024 = 43,402.34 Hz` (D211). `python tools/synth_pass_period.py fit` (test `test_settled_tables_give_two_frames_per_pass_on_every_knob`) and the Rust test `tests_pass_period::settled_tables_fit_two_frames_per_pass_on_every_knob`:

| Tables | Chain | Median `P` | Min | Max |
|---|---|---|---|---|
| settled | attack (128 knobs) | **1.9995** | 1.898 | 2.064 |
| settled | decay / release (128 knobs) | **2.0007** | 1.931 | 2.076 |
| stale sweep | attack | 2.359 | 1.944 | 5.200 |
| stale sweep | decay / release | 2.301 | 1.931 | 3.862 |

The spread of the settled fit is the display rounding (the strings carry 2 to 3 digits; 0.5 ms steps near the floor): every knob within 8 %, the median within 0.05 %. The independent readout of the oracle stays: the oscillator advances `note / (2 F)` per stage sample with 4 samples per pass, 261.59 Hz for note 60 (-0.3 cent) and 130.86 Hz for 80s Sci-Fi (+0.7 cent) ([synth-e2e-oracle-vs-native](synth-e2e-oracle-vs-native.md) s1), so one pass is `4 / (2 F)` = 2 frames = 46.08 us from the pitch alone, independent of any envelope table. The three measurements (pitch, 128 attack times, 128 decay times) agree on one period.

## 3. Per index comparison and why the clips get worse

[synth-env-rates-legacy-vs-settled.csv](data/synth-env-rates-legacy-vs-settled.csv) (`python tools/synth_pass_period.py table`): per knob the display times, the implied `P` of both tables, and the legacy / settled ratios.

| Fact | Value |
|---|---|
| Indexes where the stale table is off by more than 2 % | 104 of 128 (attack and decay / release), all inside the 5 or 6 index plateaus |
| Stale attack coefficient over settled | 1.00 .. 2.56 (the stale ramp is up to 2.56 times too fast) |
| Stale decay rate `1 - a` over settled | 1.00 .. 1.98 (the stale decay is up to 2 times too fast) |
| Never slower | the ratio is at least 0.999 at every index |

So with the stale default every filter and amp envelope that sits inside a plateau runs 1.1 to 2.6 times too fast. Band level deltas against the hardware (emulator minus hardware, dB; [emu_vs_hw](emulator-vs-hardware-audio.md), same build, default then settled):

| Clip | Filter envelope knobs | 120 to 400 Hz | 400 to 1.5 k | 1.5 to 3 k | 3 to 8 k | Score |
|---|---|---|---|---|---|---|
| `synth_12db` | filter 63 / 116, release 86 | +0.4 then +0.6 | +0.6 then +1.3 | -0.7 then **-4.0** | -0.8 then **-5.7** | 1.35 then 3.15 |
| `super_lp24_oopad` | layer filters 95 / 68 / 50 | -3.3 then -3.4 | -1.9 then -2.1 | -9.8 then -10.6 | -8.8 then -10.6 | 4.88 then 5.44 |
| `synth_leaky` | filter 20 / 61 / 45 | -1.8 then -1.8 | +0.3 then +0.3 | +2.7 then +2.8 | +3.1 then +3.5 | 1.91 then 2.02 |

Reading (EMPIRICAL, from the band deltas and the switch table of section 5):

- The slower, correct filter attack (`synth_12db` filter attack index -1: 947 ms, the stale table 690 ms) opens the native filter later, so the emulator loses 3 to 5 dB at 1.5 to 8 kHz. The hardware clip is bright there: the default envelope to cutoff path is too weak, and the stale tables compensated by opening early.
- With the firmware cutoff law on (`--synth-fw-env-law`, 127 semitones at full amount, D200 / D207) the same table switch costs `synth_12db` only 0.13 (1.44 to 1.57) instead of 1.80, which locates the 12dB Sweep regression in the default cutoff mapping, not in the table.
- `super_lp24_oopad` is already 9 to 10 dB too dark at 1.5 to 8 kHz with the stale tables; the slower filter attacks (8.2 s, 1.4 s, 0.3 s, 1.27 / 1.35 / 1.20 times slower) add another 1 to 2 dB. It is worse with the law on as well (4.01 to 4.57), so its gap is a second, separate defect of the filter path (not isolated here).
- `synth_lead`, `synth_silkpad`, `synth_horizon` are +0.04 to +0.09 (default chain) and stay within noise of the 2 digit score; the large moves of D223 for them (0.80 to 3.52) come from the law, not the tables.

## 4. Correction behind the opt-in

- `strip_map.rs`: `attack_index_with`, `decay_index_with`, `attack_seconds_with`, `decay_seconds_with` take a `RateTables`; the old functions are the `LEGACY_TABLES` case (behaviour unchanged). With the settled tables the inverse maps every index back to itself (test `settled_inverse_resolves_every_index`); the stale search returns the plateau start.
- `from_panel_with(.., tables)` and the oracle law's panel fallback (`fw_adr_law.rs`, `amp_attack_word`, `filter_attack_word`) use the inverse of their own tables. Layers with stored knobs (all 384 factory programs) never use the inverse, so no clip score changes through it.
- New switch `--synth-settled-rates` (`SynthLayer::settled_rates`, `FirmwareOptions::synth_settled_rates`): the settled tables on the default chain without the velocity attack and the 0.5 sustain of `--synth-fw-adr-law`, which also reads them. Default and `render_hash` unchanged (`DEFAULT_TABLES` stays `LEGACY_TABLES`).

## 5. Clip scores per segment

[synth-env-pass-period-clips.csv](data/synth-env-pass-period-clips.csv): `tools/emu_vs_hw.py`, the 17 synth segments of Part A (lower is better), one release build, each column a variant of the same run. `--synth-fw-resonance` is the default (D223) in every column.

| Segment | Default (stale tables) | `--synth-settled-rates` | + `--synth-fw-adr-law` | + `--synth-fw-env-law` | + both | Settled minus default |
|---|---|---|---|---|---|---|
| `organ_pipe` | 2.09 | 2.09 | 2.10 | 2.15 | 2.26 | +0.00 |
| `synth_lead` | 0.80 | 0.86 | 0.95 | 3.39 | 3.67 | +0.06 |
| `synth_crunch` | 2.08 | 2.06 | 2.18 | 2.01 | 2.07 | -0.02 |
| `synth_silkpad` | 1.36 | 1.45 | 1.49 | 4.12 | 4.23 | +0.09 |
| `synth_filterbank` | 4.69 | 4.61 | 6.49 | 4.24 | 4.36 | -0.08 |
| `synth_sineplk` | 7.47 | 7.46 | 7.43 | 7.43 | 7.41 | -0.01 |
| `synth_megaline` | 2.16 | 2.16 | 2.12 | 2.16 | 2.12 | +0.00 |
| `synth_horizon` | 2.31 | 2.35 | 2.25 | 4.21 | 4.13 | +0.04 |
| `super_lp24_horizon` | 2.31 | 2.35 | 2.25 | 4.21 | 4.13 | +0.04 |
| `super_lp24_sidechain` | 5.01 | 5.01 | 4.96 | 5.01 | 4.96 | +0.00 |
| `super_lp24_oopad` | 4.88 | 5.44 | 5.89 | 4.57 | 4.97 | +0.56 |
| `super_lp24_believer` | 1.66 | 1.64 | 1.62 | 1.61 | 1.59 | -0.02 |
| `synth_12db` | 1.35 | 3.15 | 3.38 | 1.57 | 1.77 | +1.80 |
| `synth_phat3` | 2.28 | 2.28 | 2.43 | 2.28 | 2.43 | +0.00 |
| `synth_80s` | 1.17 | 1.20 | 1.27 | 1.10 | 1.14 | +0.03 |
| `synth_leaky` | 1.91 | 2.02 | 2.68 | 2.18 | 2.75 | +0.11 |
| `pianopad_grand` | 1.82 | 1.82 | 1.81 | 1.82 | 1.82 | +0.00 |
| **mean** | **2.668** | 2.821 | 3.018 | 3.180 | 3.283 | +0.153 |

(`synth_horizon` and `super_lp24_horizon` are the same Part A moment under two names; 16 distinct clips. The D223 table reads `synth_12db` 1.38 as the "all off" build; this build, with the resonance default, gives 1.35.)

Settled tables on the default chain: **8 of 17 segments worse** (`synth_lead`, `synth_silkpad`, `synth_horizon`, `super_lp24_horizon`, `super_lp24_oopad`, `synth_12db`, `synth_80s`, `synth_leaky`), 4 better (`synth_filterbank`, `synth_crunch`, `super_lp24_believer`, `synth_sineplk`), 5 equal at two digits. The owner rule (every clip at or below its old score) fails, so the tables do not flip.

## 6. Flip proposal (not applied)

Not every segment is equal or better, so there is nothing to flip now. The one line is unchanged (`DEFAULT_TABLES = SETTLED_TABLES` in `fw_env_rates.rs`, a `--legacy-synth-env-rates` switch, re-baselined `render_hash`). Conditions for a follow-up, in order:

1. Fix the envelope to cutoff path the default chain uses (the ASSUMED level link of the firmware cutoff law, [synth-env-law](synth-env-law.md) / D223 follow-up; a sibling task runs it), so that `--synth-fw-env-law` stops regressing `synth_lead` / `synth_silkpad` / `synth_horizon`.
2. Re-run `tools/emu_vs_hw.py` with `--synth-settled-rates` as in section 5. Flip when every segment is at or below its score at the old tables (+0.01 rounding), with the law as the default or the settled tables flipped together with it.
3. Separately isolate the `super_lp24_oopad` gap (9 to 10 dB dark at 1.5 to 8 kHz in all variants).

## 7. Implications

- The settled tables and the 2 frame pass are one verified pair; the clip scores cannot demote them because the score also contains the envelope to cutoff mapping. Treat a score regression of a VERIFIED table as a defect of its consumer.
- The stale default is faster than the firmware by up to 2.6 times at 104 of 128 knobs; every default synth render keeps that error until the flip.
- `DECAY_TAU_FRACTION = 0.18` (EMPIRICAL, fitted at a plateau interior on the stale table) was never re-fitted for the settled inverse; it only touches layers without stored knobs.

## 8. Follow-ups

- Envelope to cutoff level link (blocks the flip); `super_lp24_oopad` brightness; re-score after both.
- Hold decay (knob 127): the oracle index 63 with the 0.5 threshold against the native Hold 1.0 (D212 follow-up, unchanged).
- Amp chain limit above 1 (D212 s3), unchanged.
