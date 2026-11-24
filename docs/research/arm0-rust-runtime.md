# ARM0 native Rust runtime: wall-clock per 1 ms tick

**Checked:** 2026-11-23.

**Sources:** `crates/nord-arm` + `crates/nord-tools/src/bin/arm_lle_bench.rs` (this PR); user OS v1.66 images `target/fw_unpacked/bank0_CODE.bin` (loaded at `0xC0000000`) and `bank0_SRAM.bin` (read-only, never committed); a 256 KB EEPROM image built by `tools/arm_program_load.build_image` from `target/oled/eeprom.bin` plus factory program `B3 Chords n Solo.ns4p`; Python reference `tools/arm_org_stream.py`, `tools/arm_steady.py`, `tools/arm_dsp_return.py`; [pno-jit-arm-profile](pno-jit-arm-profile.md) (Python/Unicorn cost); [dsp-to-arm-return](dsp-to-arm-return.md).

**Question:** Can ARM0 run the real OS v1.66 boot, program load and key events inside Rust below 1.0 ms wall-clock per 1 ms virtual tick (`ns4-lle2`, Gate 1a)?

## Method

Command (release build, Windows, one thread):

```
cargo run --release -p nord-tools --bin arm-lle-bench -- \
  <bank0_CODE.bin> <bank0_SRAM.bin> <eeprom_b3.bin>
```

1. Boot to the main loop `0xC00A0248` with the full board attached (panel, 4 I2C EEPROMs, UART2) and idle skipping on.
2. Real timer on, DSP port logging and the DSP return hooks installed.
3. Six phases, tick counts as in `tools/arm_org_stream.py`: load 800, idle 300, key on 40, key off 40, eight keys on 40, eight keys off 40. One tick = `200000` ARM instructions of virtual time (ASSUMED, `INSNS_PER_TICK` of `tools/arm_steady.py`) plus one timer IRQ.
4. The same phases run again on a clone with full execution (no skipping). The machine state hash (registers, CPSR, mapped memory) is compared after every phase.

## Results

Boot: 232,625,931 virtual instructions (231,624,565 executed, 999,786 skipped), 3.54 s wall. The Python reference reports about 433 M instructions for the same boot with a different counter, so the counts are not comparable.

| Phase | Ticks | Mean ms | p50 ms | p99 ms | Max ms | Ticks > 1 ms | Executed insns/tick | ARM to DSP messages |
|---|---|---|---|---|---|---|---|---|
| load | 800 | 0.407 | 0.231 | 4.373 | 5.656 | 4.8 % | 12,481 | 15,337 |
| idle | 300 | 0.211 | 0.221 | 0.371 | 0.610 | 0.0 % | 3,384 | 3 |
| key on | 40 | 0.236 | 0.234 | 0.341 | 0.448 | 0.0 % | 3,378 | 126 |
| key off | 40 | 0.221 | 0.223 | 0.358 | 0.668 | 0.0 % | 3,147 | 59 |
| 8 keys on | 40 | 0.298 | 0.226 | 0.393 | 3.133 | 2.5 % | 3,331 | 907 |
| 8 keys off | 40 | 0.288 | 0.227 | 0.884 | 2.578 | 2.5 % | 3,437 | 789 |

Full execution (no skip) costs 3.4 to 3.8 ms per tick in every phase (200,000 instructions in about 3.5 ms, about 57 MIPS). Skipping removes 94 to 98 % of the instructions: 240,468,696 skipped, 11,531,304 executed.

Equivalence: the state hash after each of the six phases is EQUAL between the skipping run and the full-execution run, and the message counts match (15,337 / 3 / 126 / 59 / 907 / 789). Idle skipping is exact on this workload, not an approximation.

## Verdict

* Steady state and key events: GO. Mean 0.21 to 0.41 ms/tick, p99 below 0.9 ms (key phases).
* Load burst and heavy key events: the worst ticks run the whole 200,000-instruction budget for real (3 to 5.7 ms). They are 4.8 % of load ticks and 2.5 % of eight-key ticks. A real-time driver must absorb them (buffer a few ticks of ARM messages, or run ARM0 on its own thread ahead of the audio clock). Meeting 1 ms on every tick needs about 200 MIPS (3.5x the interpreter).

## What the over-1-ms ticks mean for real time

Plainly: **4.8 % of load ticks and 2.5 % of key-event ticks exceed 1.0 ms wall-clock per 1 ms virtual tick** (2.5 % in both eight-key phases). Single-key and idle phases have none. In every phase the mean is far below 1 ms, so the work is a throughput surplus with rare spikes, not a hard miss on average:

* **Amortization works if ARM0 is decoupled.** ARM0 runs ahead of the audio clock on its own thread and queues its ARM to DSP messages. A burst of 3 to 6 ticks at 3.5 to 5.7 ms is repaid by the following 0.2 ms ticks (mean 0.21 to 0.41 ms leaves 60 to 80 % of the budget free). It costs a few ms of message latency, not an audio dropout, because the SHARC side consumes the queue, not the ARM tick.
* **A hard miss happens only if ARM0 is locked 1:1 to the audio callback** (a tick must finish inside its own 1 ms). Then 2.5 to 4.8 % of ticks glitch. Do not wire it that way before a faster core exists.
* **Not measured:** the longest run of consecutive over-budget ticks (needed to size the queue) and message latency under a queue. Follow-up.

**Go / no-go for ARM0 real-time: GO**, conditional on the decoupled thread plus message queue. NO-GO for a lockstep design until the interpreter reaches about 200 MIPS (predecode cache).

## Not verified

* `INSNS_PER_TICK = 200000` is ASSUMED. If the real ARM926 retires fewer instructions per ms the heavy ticks shrink.
* The DSP return layers beyond the SY idle mask and the shadow of the ARM's own messages (live chip, initial image) are not ported: `ns4-lle5`. The bench does not read the SPI return path in the key phases enough to cost anything measurable.
* Boot null-call count 747 (Rust) against 762 (Unicorn run of `arm_steady.py`); cause not found (likely instruction-count or poll-site semantics at the poll breaker).
* Load-phase message count 15,337 against 7,340 in the Python research; the boot timing differs and the Python run used another stop point. Not investigated.
* Only OS v1.66 `bank0` (ARM0) and one program were measured.

## Implications

* A native interpreter is enough for the mean and steady state; decision [D244](../DECISIONS.md).
* Idle skipping needs no firmware knowledge, so it keeps working on other OS images.
* Real-time use needs tick-burst absorption or a faster core, not Unicorn.

## Follow-ups

* Predecode cache (or a threaded interpreter) to reach about 200 MIPS and remove the over-1-ms ticks.
* Measure the distribution of non-idle instructions per tick on real hardware traces to replace the ASSUMED 200,000.
* Find the 747 against 762 null-call difference and the load-phase message count difference.
* Port the live-chip and initial-image return layers (`ns4-lle5`).
