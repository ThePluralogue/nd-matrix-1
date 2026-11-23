# SMP bus tap: Simple Tune (piano layer B) and the silent ring tap (OQ41, D233)

- **Checked:** 2026-11-23
- **Sources (local, never committed):** SMP image `target/oracle/smp.bin`, Factory Restore RevK v1.66 `.ns4b`, `factory_programs/Program/Bank E/Simple Tune.ns4p`, `Bank B/Gradient Whl.ns4p`, `Bank A/Astoria Grand.ns4p`; live runs of `tools/arm_dsp_live.py --bus-tap ADDR --bus-side --log DIR` with `sharc-live` (new `B` command). Earlier notes: [piano-smp-pitch-outliers](piano-smp-pitch-outliers.md) s3, [piano-release-voice](piano-release-voice.md), [piano-bus-convention](piano-bus-convention.md) s2, [smp-voice](smp-voice.md) s2.
- **Question:** (1) can Simple Tune (layer B only, bus `0x9405A`) be ranked; (2) does a bus tap also explain the silent ring tap after frame ~3,800 of the held-note run of D226, and what does the release voice do in the bus?

Tags follow `.agents/rules/provenance-in-code.md`: **VERIFIED** = reproduced by a test or a command below, **EMPIRICAL** = measured in this harness (one run each), **ASSUMED** = reading.

## 1. Short answer

| Question | Result | Tag |
|---|---|---|
| Simple Tune tap | **Ranked.** Bus `0x9405A`: audio, held-note peak -27.00 dBFS (frames < 5,500), pitch 0.01205 cycles per frame = `2 * 0.00603` (layer B octave +1), peak with the key-up voice -22.95 dBFS. Row filled in `program-ranking.csv`, flag `piano_chain_no_audio` removed (153 of 155 piano programs have a tap; the 2 left are the zone gain 0 programs) | EMPIRICAL (`smp_bus_rank.py`; `smp_pitch_audit.py`: 148 `ok`, 5 `octave_shift`, 0 `mismatch`) |
| Is the bus tap the same signal as the ring? | Yes for layer A: bus `0x9404E` against the ring tap of the same run (Gradient Whl). While the ring is alive the peaks agree (ratio 1.00000) and the correlation is 0.977; the rest is the post stage one-pole smoothing and the second bus set `0x9403E` | EMPIRICAL |
| Why is the ring tap silent after a few thousand frames? | **The ring gate closes while the voice goes on.** Same run, same frames: Astoria Grand ring alive frames 106..2,316, then 0.0003 and 0; the bus `0x9404E` carries the held note to the end of the run (0.1568 down to 0.049). Gradient Whl: ring alive 106..2,384, bus signal in all 5,968 frames. The voice does not stop, the post stage output does | EMPIRICAL (two programs, side file beside the ring tap) |
| Release voice in the tap | **Measured in the bus** (Astoria Grand, key 3,000 ticks, 4,000 ticks after the key-up): key-up at about frame 5,600, release peak 0.1798 (-14.91 dBFS) at frame 7,297, window RMS 0.1146 against 0.051 to 0.053 before the key-up: **+6.9 dB**; D226 predicts +6.5 dB from the record words. The release is as loud as the note onset (-16.09 dBFS) | EMPIRICAL (one run; levels of different sample passages, so the dB agreement is loose) |

## 2. The tap

`sharc-live` command `B ADDR [i|f [PATH]]` (`runner/live_bus.rs`, `SyModel::bus_tap`):

| Form | Effect |
|---|---|
| `B ADDR` | the tapped frame of every audio interrupt is the DM word pair `ADDR`, `ADDR+1` (signed integer words x `2^-29`), replacing the ring frame |
| `B ADDR i PATH` | the bus frames go to `PATH` (f32 little endian `L R`); the ring tap goes on, so both are read in the same run |
| `B ADDR f ...` | the words are float bits already (no scaling) |
| `B 0` | off. Off is the default: one `Option` test per audio interrupt, no file, no read |

The scale is the post stage conversion `FLOAT BY -29` (VERIFIED, [piano-bus-convention](piano-bus-convention.md) s2b: ring = bus x `2^-29`, unity). Harness: `arm_dsp_live.py --bus-tap ADDR [--bus-side] [--off-ticks N]` writes `smp_bus.f32` beside `smp_tap.f32`; `tools/smp_bus_rank.py` turns a bus file into the `chain.jsonl` record of `program_rank_smp.py`. Tests: `live_bus` (4), `live_sy_tests` (2: replace and side file), `live_tests` (`B` protocol), `tools/tests/test_arm_smp_flash.py` (2), `tools/tests/test_smp_bus_rank.py` (4).

## 3. Simple Tune

Layer B only, EP5 BrightTines XL, octave +1, core 2 ([piano-smp-pitch-outliers](piano-smp-pitch-outliers.md) s3). Run: key 60, 3,000 ticks, `--fifo-pace 1`, `--bus-tap 0x9405A --bus-side`; 3 of 3 descriptors, `law_ok` 3.

| Frames | Bus peak |
|---|---|
| 0..500 | 0.0447 (note onset) |
| 500..5,500 | 0.041 falling to 0.031 |
| 5,500..5,968 | key-up voice: 0.0242 (dip), 0.037, 0.054, 0.0712 (still rising when the 300 tick off phase ends) |

The ring over the same run: 0 of 11,936 words nonzero (`kind silent`), as in D230. L equals R in every frame (mono layer). The CSV peak is the held note (frames < 5,500): the ring rows are held notes too (their gate closes before the key-up). Peak of the whole file, key-up voice included: -22.95 dBFS (still rising). Pitch 0.01205 = 2.00 x 0.00603: the layer octave +1; `smp_pitch_audit` reads `octave_shift` like the four layer A rows.

## 4. The silent ring tap of D226

D226 saw the ring silent "from about frame 3,800" in the 8,000 tick held-note run. In the runs here (same program, 8,000 ticks, and a 3,000 tick run) the ring is alive up to frame 2,316 and then zero (500 frame peaks `0.0486, 0.0003, 0, ...`); the bus holds the note throughout (0.1569, 0.1695, 0.1714, 0.1457 ... 0.0494 at frame 13,800). So the silence is the post stage output, not the voices: the ring gate closes after about 2,300 frames (53 ms) on both programs tried. The 3,800 of D226 was read from a coarser window. What closes the gate (the words `DM(0xB2031)`, `DM(0xB2032)` and the ARM message that sets them) is not identified. The ring tap of the 152 layer A rows is therefore the first ~2,300 frames of the note; the bus tap reads the whole note and the release.

## 5. Release voice of Astoria Grand (bus `0x9404E`)

| Item | Value |
|---|---|
| Key-up | pass 3,526 of the run; frame ~5,600 of the tap (the voice dips at 5,750 as `F11 = F12 = 0` reset the note-on voices, D226) |
| Held note before key-up | window RMS 0.0532 (frame 5,250), 0.0510 (5,500); peak 0.109 |
| Release | window RMS 0.0605 (6,250), 0.0956 (6,750), 0.1146 (7,250); peak 0.1798 at frame 7,297 (39 ms after the key-up) |
| Decay | RMS 0.0795 (8,750), 0.049 (10,000), 0.0275 (12,000), 0.0214 (12,500) |
| Release RMS peak over held note | +6.9 dB (window 7,250 against the mean of 5,250 and 5,500) |
| D226 prediction | key-up gain `0.21875 * 1.0` against the note `0.21875 * 0.4719`: +6.5 dB |

Reading: the firmware key-up is as loud as the note (peak -14.9 dBFS against a note onset of -16.1 dBFS, RMS 6.9 dB over the held note at key-up). That supports the firmware ratio of D226 (`--piano-keyup-fw`) against the native default `0.30 * level` (17 dB lower for this program). It is one run of one program, and 6.9 against 6.5 compares different passages of the sample. The opt-in stays (no flip; the reasons of D226 stand).

## Reproduce

```bash
M=target; B=".reference/Nord Stage 4 Factory Restore RevK v1.66.ns4b"
cargo build --release -p nord-sharc --bin sharc-live
uv run --with unicorn --with capstone python tools/arm_dsp_live.py $M/fw_unpacked/bank0_CODE.bin \
    $M/fw_unpacked/bank0_SRAM.bin --eeprom $M/oled/eeprom.bin --program "factory_programs/Program/Bank E/Simple Tune.ns4p" \
    --live $M/release/sharc-live --poll-advance 64 --ns4b "$B" --out run.json --chip "PNO:1=pno=$M/PNO/core1_sram.bin" \
    --chip "SMP:0=smp=$M/oracle/smp.bin" --piano-bind "$B" --log LOG --key-ticks 3000 --bus-tap 0x9405A --bus-side
python tools/smp_bus_rank.py --bus LOG/smp_bus.f32 --entry "Program/Bank E/Simple Tune.ns4p" --note-frames 5500 --out chain.jsonl
python tools/program_rank_smp.py --chain chain.jsonl --csv docs/research/data/program-ranking.csv
# release: Astoria Grand, --bus-tap 0x9404E --bus-side --key-ticks 3000 --off-ticks 4000
```

## Implications

- Layer B has a measurable tap; the 33 two-layer programs need one run per layer B bus (the bus address is `rec[-18]` of the voice record, `0x9405A` for Simple Tune) to rank the second layer.
- A level verdict of the piano against the native render must use the bus, not the ring, for anything after the first ~2,300 frames.
- The release level question of OQ41 is answered for Astoria Grand (+6.9 dB over the held note).

## Follow-ups

1. Which words close the post stage gate after ~2,300 frames (watch `DM(0xB2031..0xB2032)` with the `W` command).
2. Layer B of the 33 two-layer programs (bus address per program) and a second release program (low velocity key-up, `F15` below 1.0).
3. The dry native piano render for the level (D216).
