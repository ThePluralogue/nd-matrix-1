# Whole-Pass AOT and multi-core SHARC pass benchmark at 48 kHz: verdict NO-GO

Checked: 2026-11-24
Sources: ORG/PNO Core1/Core2 SRAM images (local-only, `target/sram/{ORG,PNO,FX}/`, produced from `.reference/` as in `org-system-run.md`); ORG Core2 messages `docs/research/data/org-system-load-core2.script` flattened to pass 0 plus `--msg 0:0x3e021=1`; `crates/nord-sharc/src/core/{aot_whole,aot_mem,aot_emit,aot_lower,aot_hot,aot_glue}.rs`, `runner/pass_sched.rs`, `sharc-aot --whole-pass`, `sharc_jit_diff --aot-pcs/--aot-top/--bench`, `crates/nord-tools/src/bin/lle_sharc_bench.rs`; baselines `sharc-aot-multi-block.md` (PR #466 / D118), `pno-jit-arm-profile.md`; bean `ns4-lle3`.
Question: can Whole-Pass AOT (`sharc-aot` generates Rust, `rustc -O` compiles it into the binary) with native lowering of memory, SIMD, register-move and shift instructions, run on a 1-thread-per-core spin-barrier scheduler, execute every SHARC core of the Nord Stage 4 inside the real-time budget of 46.08 us per pass (2 frames at 43 402.34 Hz, ASSUMED in `org-system-run.md`)?

## Result in one paragraph

**NO-GO on this prototype.** The whole-pass AOT is bit-exact (ORG Core2: 57 blocks, boot 100 000 steps + 300 passes = 1 854 395 steps matched 1:1, full state and SRAM compared every 20 000 steps) and faster than the interpreter, but only by x1.3 to x1.4 per pass. The slowest measured core (ORG Core2, 6 280 steps per pass) needs **~370 us per pass with AOT, 8.0x the budget** (repeat runs 8.0 to 9.8x; the interpreter needs 12 to 13x). PNO Core2 (4 266 steps) needs 7.2 to 7.9x on the interpreter. Real time needs 6 280 steps / 46.08 us = **136 Msteps/s per core**; the AOT delivers 10 to 17 Msteps/s. The fast-f32 mode is no faster than exact mode (explained below). Closing the gap needs roughly a further 8x on the hottest core, which the structure of the current lowering cannot reach without the changes under "What would close it".

## 1. Whole-Pass AOT prototype

| Item | Result |
|---|---|
| Block discovery | pcs executed by the real pass (`--aot-pcs`, 4 301 pcs for ORG Core2) cut into blocks at flow instructions plus their 2 delay slots, DO, stack ops and idle; batches never start inside delay slots |
| Native lowering | `DirectMem`, `DagMem` (pre/post-modify, immediate offsets), SIMD twin writes and broadcast loads, `Move(Other/RegSwap)`, `ShiftImm`, `Multifunction`, immediate loads, via `MemXfer` (`core/aot_mem.rs`) |
| Generated module | ORG Core2: 57 blocks, 210 chunk functions of up to 24 items, 3 637 native memory/move sites, 466 generic `aot_step` fallback sites, ~1.1 MB of Rust (never committed: derived from firmware, D20/D128) |
| Exactness | unit test `xfer_equals_the_interpreter_on_random_states` (about 20 instruction shapes, scalar and SIMD, 23 seeds each, full register file and memory compared) and the real pass diff: `sharc_jit_diff ... --aot-top 999 --chunk 20000` printed `AOT DIFF OK: 57 blocks, boot 100000 steps + 300 passes (1854395 steps) matched 1:1`. Run-time coverage 85.4% of audio-pass steps inside blocks, 3.4% of them native compute (`pe`) |
| Per-pass gain | `--bench`: interpreter 0.851 ms/pass (7.38 Msteps/s), AOT 0.634 ms/pass (9.91 Msteps/s), **x1.34**; inside the blocks 6 370 steps took 896 us interpreted vs 546 us AOT (x1.64, ~100 ns per step: far from native speed) |

Compile time is the practical limit (EMPIRICAL, this host, 32 logical cores): inlining the whole `step` body at all 4 103 sites ran > 50 CPU-minutes and was aborted; one function per block with 3 600 inlined transfers took 12.5 min; chunking into 24-item functions with out-of-line `MemXfer` handling builds the `nord-sharc` crate with the module in about 2 min 15 s. Five images would add ~10 min of `rustc` per release build, so AOT builds use a separate target dir (`CARGO_TARGET_DIR=target/aot`) and ordinary builds stay firmware-free.

## 2. Multi-core benchmark

`lle_sharc_bench` runs one OS thread per core. The coordinator publishes a tick number in one atomic, each worker spins on it, runs one pass and counts in on a second atomic, the coordinator spins until all have counted in. No mutex, no channel, no allocation in the loop (`runner/pass_sched.rs`). Passes: 16 warm-up + 300 timed, ARM messages delivered before the pass they name. ORG Core2 uses the 912 flattened ARM messages; PNO cores have no message script yet (idle-state passes).

Host: Windows 11, 32 logical cores, `--release`. Latency is the wall time of the pass only (message delivery excluded). Values come from one run; repeats varied by about +-15%. Two earlier runs on a busy host were 2x slower and are discarded.

Exact 40-bit mode, 3 cores concurrently (PNO c1, PNO c2, ORG c2):

| Core | Engine | Steps/pass | Mean us | p50 us | p99 us | Worst us | Mean / 46.08 us |
|---|---|---:|---:|---:|---:|---:|---:|
| PNO Core1 | interpreter | 1 214 | 74.0 | 71.9 | 112.8 | 117.4 | x1.6 |
| PNO Core2 | interpreter | 4 266 | 332.9 | 319.3 | 520.0 | 584.7 | x7.2 |
| ORG Core2 | AOT | 6 280 | 367.6 | 360.6 | 557.3 | 662.9 | x8.0 |
| tick (slowest core + barrier) | | | 373.8 | 361.9 | 582.4 | 664.4 | x8.1 |

Total CPU per tick 774.5 us = 16.8 budget periods of one host core. The same three cores, all interpreter: ORG Core2 470.9 us (x10.2), PNO Core2 364.9 us, PNO Core1 84.3 us, tick 483.2 us, total CPU 920 us (20.0).

ORG Core2 alone (300 passes each): interpreter 585 / 553 us; AOT 450 / 437 / 410 / 370 us.

Fast-f32 mode (`--tier approx`, module `ORG2F`), 3 cores: ORG Core2 mean 468.8 us, p99 996 us, tick 483 us, total CPU 921.6 us (20.0); alone 463.7 to 471.2 us. **No faster than exact mode.** The approximate kernels only replace the native `pe` compute batches, which are 3.4% of the executed steps; the other 96.6% (memory, move, flow) run the same code in both tiers. Fast-f32 is also not bit-exact, so it would need a listening/null test; it buys nothing until the compute share of the pass is native.

Jitter: p99 is 1.5x to 2.5x the mean for the 300 to 450 us passes and the worst pass is 1.8x the mean, so a real-time budget needs headroom above the mean, not only below it.

### Not measured (blockers, listed on purpose)

| Core | Why |
|---|---|
| ORG Core1, FX Core1/Core2 | the `org` and `pno` presets fault at boot on these images (`unimplemented at 0x1286f4` for FX Core2, `0x121182` for ORG Core1 with the PNO preset): their entry pcs, gates and message scripts are not in `live_presets.rs`. Needs their own presets (follow-up) |
| SY1SY2, SMP | interrupt-driven (`live_sy`, `live_smp`), no poll loop: `pass_sched` runs poll-loop passes only. Needs the `SyModel` ISR driver as a job type (follow-up). Their step counts per audio interrupt come from `sy-smp-audio.md`, not from this benchmark |
| 128/256/512-frame buffers | a buffer of N frames is N/2 passes, so the per-pass budget is unchanged (512 frames = 256 passes in 11.8 ms). Larger buffers only absorb jitter, they do not lower the throughput needed |

## 3. Risks found

| Risk | Finding |
|---|---|
| Compile time | 12.5 min to > 50 CPU-min for naive emission; ~2 min per image with chunking; cannot go into the default build or CI |
| Dynamic SRAM code invalidation | SRAM code can be rewritten at run time. Block entry memcmp-checks the code snapshot (`aot_code_matches`), and the native store path invalidates decode-cache slots only inside the SRAM code extent. **Known hole:** self-modifying code *inside* a running block is not detected until the next entry. Not seen in the ORG/PNO passes (the 1:1 diff would fail), unproven for SY/SMP overlays |
| Computed jumps | `JUMP (Md,Ix)`, register `CALL` and `RTS` end the block and return to the dispatcher; delayed flows still run through `Core::step`. 466 of 4 103 sites stay generic (`aot_step`), mostly flows, DO setup and conditional compute |
| Dispatch cost | each block entry does a binary search, statistics and a code memcmp; the 300-pass diff run counted 25 126 block calls, 8 650 misses and 11 699 bails (about 29 bails per pass) that fall back to the interpreter |
| Fast-path guard | the native memory path needs no peripherals, no read watch, no site-hit capture; any of them silently turns the AOT off for the core (`aot_fast_mem`) |
| Mode divergence | fast-f32 is not bit-exact and has no listening test |

## What would close it (not built here)

Real time needs about 8x more than the AOT has, at ~100 ns per AOT step today:

1. Emit each memory/move site as straight-line Rust with constants folded (address mode, register numbers, SIMD flag known at generation time) instead of calling the generic `aot_xfer(&MemXfer)`. Largest item; it also grows compile time.
2. Keep the register file and DAG registers in locals across a whole block (the `Pe` idea of `sharc-aot-multi-block.md`, extended to memory ops), so loads and stores do not round-trip through `Core`.
3. Native `JUMP/CALL/RTS` with a static target table and native `DO` loops, so most bails disappear.
4. Replace the per-entry memcmp with a code-generation counter and the binary search with a direct table.
5. Skip work: idle fast-forward of poll loops, and not running cores whose output is unused.

A combined x8 from items 1 to 4 is a stretch and costs ~10 min of `rustc` per build. Options for `ns4-lle4` and later: (a) treat SHARC LLE as non-real-time (render-ahead/offline) and keep the HLE engines for live play; (b) run items 1 to 3 as a bounded experiment on ORG Core2 with a checkpoint of <= 180 us/pass (x4) before committing Gate 2 to real-time SHARC; (c) more threads do not help: each core already has its own thread and the slowest core sets the tick.

## Implications

- Gate 2 beans (`ns4-lle4` and later) must not assume real-time SHARC execution. Measured gap on the slowest benchmarked core: x8.0 with AOT, x10 to 13 interpreted. All benchmarked cores together need 17 to 20 host cores of budget.
- `pass_sched` and `lle_sharc_bench` are reusable for `ns4-lle4`/`ns4-lle5` and for the follow-up experiment.
- D118 (whole-pass AOT) is answered: bit-exact and buildable, not real time.

## Follow-ups

- Presets and message scripts for ORG Core1 and FX Core1/Core2; SY1SY2/SMP as ISR-driven jobs in `pass_sched`, so the 300-pass benchmark covers all five chips.
- Bounded experiment on items 1 to 3 with an x4 checkpoint.
- Listening/null test for the fast-f32 tier, only if the compute share of the pass becomes native.

## Reproduce

```bash
# images and message scripts: see org-system-run.md (local-only)
cargo run --release -p nord-sharc --bin sharc_jit_diff -- IMG ARGS --aot-pcs target/lle/org2.pcs
cargo run --release -p nord-sharc --bin sharc-aot -- IMG --whole-pass target/lle/org2.pcs --force 0x125DDF --tier pe --out target/lle/org2_whole.rs
export NORD_SHARC_AOT_HOT="ORG2=$PWD/target/lle/org2_whole.rs;ORG2F=$PWD/target/lle/org2_whole_approx.rs"
CARGO_TARGET_DIR=target/aot cargo build --release -p nord-tools --bin lle_sharc_bench
target/aot/release/lle_sharc_bench --engine aot --passes 300 \
  --core PNOc1=target/sram/PNO/core1_sram.bin,pno \
  --core PNOc2=target/sram/PNO/core2_sram.bin,org \
  --core ORGc2=target/sram/ORG/core2_sram.bin,org,target/lle/org2.msgtokens,ORG2
```
