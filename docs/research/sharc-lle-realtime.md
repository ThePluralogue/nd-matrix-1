# Whole-Pass AOT and multi-core SHARC pass benchmark at 48 kHz: verdict NO-GO

Checked: 2026-11-24 (bounded experiment: same day)
Sources: ORG/PNO Core1/Core2 SRAM images (local-only, `target/sram/{ORG,PNO,FX}/`, produced from `.reference/` as in `org-system-run.md`); ORG Core2 messages `docs/research/data/org-system-load-core2.script` flattened to pass 0 plus `--msg 0:0x3e021=1`; `crates/nord-sharc/src/core/{aot_whole,aot_mem,aot_emit,aot_lower,aot_hot,aot_glue}.rs`, `runner/pass_sched.rs`, `sharc-aot --whole-pass`, `sharc_jit_diff --aot-pcs/--aot-top/--bench`, `crates/nord-tools/src/bin/lle_sharc_bench.rs`; baselines `sharc-aot-multi-block.md` (PR #466 / D118), `pno-jit-arm-profile.md`; bean `ns4-lle3`.
Question: can Whole-Pass AOT (`sharc-aot` generates Rust, `rustc -O` compiles it into the binary) with native lowering of memory, SIMD, register-move and shift instructions, run on a 1-thread-per-core spin-barrier scheduler, execute every SHARC core of the Nord Stage 4 inside the real-time budget of 46.08 us per pass (2 frames at 43 402.34 Hz, ASSUMED in `org-system-run.md`)?

## Result in one paragraph

**NO-GO on this prototype.** The whole-pass AOT is bit-exact (ORG Core2: 57 blocks, boot 100 000 steps + 300 passes = 1 854 395 steps matched 1:1, full state and SRAM compared every 20 000 steps) and faster than the interpreter, but only by x1.3 to x1.4 per pass. The slowest measured core (ORG Core2, 6 280 steps per pass) needs **~370 us per pass with AOT, 8.0x the budget** (repeat runs 8.0 to 9.8x; the interpreter needs 12 to 13x). PNO Core2 (4 266 steps) needs 7.2 to 7.9x on the interpreter. Real time needs 6 280 steps / 46.08 us = **136 Msteps/s per core**; the AOT delivers 10 to 17 Msteps/s. The fast-f32 mode is no faster than exact mode (explained below). Closing the gap needs roughly a further 8x on the hottest core, which the structure of the current lowering cannot reach without the changes under "What would close it". A follow-up gap analysis and two more lowering rounds (section "Gap analysis and lowering rounds") moved the best ORG Core2 case only from x8.0 to x7.0; the curve flattens far above x2.

**Bounded redesign experiment (section "Bounded redesign experiment", later the same day): ORG Core2 x7.2 to x4.6 of the budget (fast-f32 x4.0), still bit-exact; stopped at the pre-set checkpoint (x3.5) because it was not reached and the remaining steps do not reach x2 in exact mode.** PNO Core2 (interpreter, no AOT module built for it) is now the slowest benchmarked core at x6.7.

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

## Gap analysis and lowering rounds (follow-up, same PR)

Method: `lle_sharc_bench --profile` (`core/aot_prof.rs`, `rdtsc` around every fallback step, every block call and every compute field; off by default). ORG Core2, one core, 316 passes (16 warm-up), the shipped module. Profile overhead inflates the pass (about 1.4 to 1.9 M cycles per pass against ~1.0 M unprofiled), so read the shares, not the absolute cycles.

**Why "3.4% native" was misleading.** The 3.4% counted only the `pe` compute batches. The memory, move and shift transfers lowered by `aot_mem.rs` were not counted as native: they are 76.6% of the steps (4 810 of 6 280 per pass). The problem is not coverage, it is the cost of a lowered step.

Where the pass time goes (ORG Core2, exact tier, round 2 build; shares of pass time):

| # | Cause | Steps/pass | Share of time | Note |
|---|---|---:|---:|---|
| 1 | Lowered transfers, memory/move/DAG glue (`aot_xfer` + `aot_tail`), compute excluded | 4 810 | ~37% | about 100 to 150 cycles per step: generic `MemXfer` match, `Core` field traffic, sram bounds checks |
| 2 | Compute fields inside lowered transfers (2 752 per pass): 1 981 on a generated `Pe` kernel, 771 on `exec_compute_op` | 2 752 ops | 29% | **140 to 210 cycles per op for both paths** (includes ~40 cycles of profile overhead): the exact 40-bit float arithmetic itself, not the dispatch, dominates; `Pe` load/store of ~170 bytes per op adds on top |
| 3 | Instructions after a bail-out, run by the interpreter until the next block start (`Move:Other(4a)` 5.3%, `DagMem` 3.1%, `DirectMem` 2.0%, `ComputeBoth` 1.8%, `LoadImm` 1.6%, `Move:Other(15b)` 1.4%, ...) | 876 | 18.3% | all of the "unblocked" steps are mid-block: 29 bails per pass |
| 4 | `lean_pass_aot` loop glue and idle check | | 9.6% | partly profile overhead |
| 5 | Generic `aot_step` inside blocks (`Move:Other(4b)` 1.5%, `DirectMem` 1.4%, `ComputeCond` 0.5%, `Flow:JumpIndirect` 0.7%) | 284 | 4.5% | forms `MemXfer` does not lower (conditional, long-word) and flows |
| 6 | Block entry/exit dispatch (search, guard, code memcmp) | 83 calls | 1.2% | |
| 7 | Entry refused by the guard | 24 | 0.3% | all "hardware loop ends inside block" |

No cost came from computed jumps (33 `JumpIndirect` steps per pass, 0.7%) or from SRAM code invalidation (no "SRAM code changed" refusal in 300 passes).

**The curve (ORG Core2, one core, mean us per pass, best of the repeats, noisy host: repeats vary +-15% and busy-host runs were up to 1.7x slower).** The shipped state was re-verified at the end: `AOT DIFF OK: 57 blocks, ... 1854395 steps matched 1:1`, `--bench` x1.364 (interpreter 0.871 ms, AOT 0.639 ms per pass). Every row except the interpreter was checked 1:1 against the interpreter with `sharc_jit_diff --aot-top 999 --chunk 20000` (boot 100 000 + 300 passes, 1 854 395 steps).

| Build | Lowered share of steps | Best us/pass | x budget (46.08 us) | `--bench` speedup vs interpreter |
|---|---:|---:|---:|---:|
| Interpreter | 0% | 531 to 585 | 11.5 to 12.7 | 1.00 |
| v1: transfers lowered, compute through `exec_compute_op` | 76.6% | 370 | 8.0 | x1.34 |
| v2: compute fields of transfers on generated `Pe` kernels (one out-of-line kernel per distinct op, 110 kernels; inlining one `Pe` per site made `rustc` use 19 GB and was killed) | 76.6% | 322 | 7.0 | x1.38 |
| v3: blocks cut after every guard-changing instruction (795 blocks, no bail-out tail) | 81.8% | 377 | 8.2 | x1.14 |

v3 raised the covered share to 97% of the steps but dispatch grew from 1.2% to 9.2% of the time (each entry searches, checks the guard and memcmps the code), so it was **not faster and was reverted** (not in the PR; the test for it went with it). Two rounds after v1 moved the best case from x8.0 to x7.0. **The curve flattens far above x2**: stopping this line as the orchestrator asked. What the profile says would still be needed is a different design, not a third round: per-site constant-folded code with the register file in locals for a whole block (rows 1 and 3), and compute that is cheaper than ~100 cycles per 40-bit op (row 2), where only the fast-f32 tier can help.

**Fast mode.** Exact and fast-f32 now share one lowered code path: the generated module differs only in the kernel functions' `Kern::Exact` / `Kern::Approx` argument (`aot_pe_both`), chosen at generation time (`--tier pe|approx`). Measured with the shipped v2 build, final re-run on a busy host (interpreter 643 us): ORG Core2 exact 502 / 527 / 488 us against fast-f32 378 / 365 / 378 us (about 25% faster, x7.9 to x8.2 of the budget); an earlier session gave exact 444 / 449 / 502 against fast 350 / 409 / 433 us (10 to 20%). Fast-f32 gains because compute is 29% of the time and the host-`f32` kernel is cheaper than the exact 40-bit one; it stays far from real time. **Not done:** the Fast-vs-Exact error in dB on a real pass for this build. The earlier approximate tier measured 112 to 141 dB SNR (window values) on the 8-voice pass, worst measured pass 103.8 dB (`sharc-aot-native-float.md`); `sharc_jit_diff --aot-audio --buf ...` with the approximate module compiled first would reproduce it, but the multi-module build embeds the exact module first. Fast-f32 stays opt-in until that number exists.

### Missing cores: what each needs

| Core | Result here | Smallest fix |
|---|---|---|
| ORG Core1 | boots under the `org` preset but no pass returns to the poll loop (hits the 2 000 000-step cap, idle false) | it is a Core1 section: start it by the P4I interrupt like PNO Core1 (`irq`), with its own idle pc; both need to be read from its listing (not derived here) |
| FX Core1, FX Core2 | the `org`/`pno` presets fault at boot | `FX1`/`FX2` in `runner/fx_run.rs` already hold entry `0x1211E3`, idle pcs `0x12128B` / `0x12125B`, triggers, devs and the Core1 start by `irq (15, 0x41)`: add `fx1`/`fx2` presets built from them, and call `FxPeer::before_pass` (the Core1 hand-over counter Core2 waits for) as a per-tick hook in `pass_sched` |
| SY1SY2, SMP | not poll-loop passes | a `PassJob` kind that runs the `live_sy` / `live_smp` interrupt pass (`SyModel`) instead of `lean_pass`; presets exist (`sy_preset`, `smp_preset`) |

### Total CPU across the eight cores

Measured (3 cores together, exact mode, v1 AOT build): PNO Core1 74 us, PNO Core2 333 us, ORG Core2 368 us. The other five are not measured. ASSUMED for the projection: ORG Core1 equals ORG Core2 (same organ program, `dsp-image-hunt.md` 5.17), FX Core1/2, SY1SY2 and SMP each equal PNO Core1 (74 us; probably optimistic for FX, which `fx-contexts.md` shows busier). Sum: 74 + 333 + 350 + 350 + 4 x 74 = about 1 400 us per 46.08 us period = **about 30 budget periods of host CPU**. Fraction of real time each host can reach (the slowest core, about 350 us, is a floor on one core's pass time whatever the core count; the packing figure is sum / host cores):

| Host cores for the emulator | Wall us per period | Fraction of real time |
|---:|---:|---:|
| 4 | ~350 (sum / 4 = 350) | ~13% |
| 6 | ~350 (sum / 6 = 234, floored by the slowest core) | ~13% |
| 8 | ~350 (floored by the slowest core) | ~13% |

With every host from 4 cores up the slowest core, not the core count, decides: real time needs it ~7.6x faster even with unlimited cores. Only PNO Core1 (x1.6 measured) is within reach of one core of real time.

## Bounded redesign experiment (ns4-lle3 follow-up 2)

Question: can ORG Core2 get from x7.0 to at most x2 over budget, bit-exact in exact mode, by (1) per-site folded code, (2) the register file in locals, (3) native control flow and (4) less per-step bookkeeping? Checkpoint set beforehand: after the first two steps the best core must be at least x2 faster than the AOT before (about x3.5), else stop.

Method. Host: AMD Ryzen 9 5900XT (16 cores, 512 KB L2 per core, 32 MB L3 per CCD), Windows 11, other sessions building and running at the same time, so a single run varies by +-15% and more. All "min" figures are the minimum mean of 10 runs of `lle_sharc_bench --engine aot` on ORG Core2 (300 passes, 16 warm-up, 912 ARM messages), binaries kept per step and run alternately. Every step was verified `AOT DIFF OK ... 1854395 steps matched 1:1` (boot 100 000 steps + 300 passes, state and SRAM every 20 000 steps) and the unit tests (`cargo test -p nord-sharc --lib`). Tags: EMPIRICAL for every timing.

### What the profile showed first

| Finding | Evidence |
|---|---|
| The 29 bail-outs per pass were not delay slots. Two big blocks (`0x1273c9`, 676 steps, and `0x127cda`, 958 steps) left the native path at step 51 and 824: a `BIT SET IRPTL` (software interrupt) and a mode write, which `aot_sig` correctly treats as guarded state. They put ~760 steps per pass (12%) on the interpreter. | temporary print of block, step, pc and guard fields on the 300-pass diff (12 300 bails); `T18a` word `1402 0020 0000` at `0x127434` |
| An interpreter step and an AOT step cost about the same at that point (~100 cycles), so removing the bail-outs did not move the time (best of 6: 299 us before and after). | step 0 below |
| The generated module was 512 k x86 instructions (~2.5 MB) for 3 637 transfer sites. A pass runs each site once, so it fetches ~2.2 MB of code against 512 KB of L2. | `rustc --emit asm` instruction count of the module; exe size 4.94 MB with the module, 2.44 MB after step 1 |
| Of the ~123 cycles per compute op on the generated `Pe` kernel, about half was copying the ~170-byte element state in and out of `Pe`. | micro-benchmark: `aot_pe_both` (both elements) 84 ns, `Pe::exec` alone 13 ns per element |

### Steps and measurements

| Step | Change | Min mean us | x budget | `--bench` speedup vs interpreter |
|---|---|---:|---:|---:|
| Before (PR #660 build) | whole-pass AOT as merged | 333.5 | 7.24 | x1.27 to x1.36 |
| 0. Fewer bail-outs (commit `perf(sharc): AOT blocks end at guard-changing instructions`) | a system register write or bit operation ends its block (273 blocks); a store re-checks the guard only after an access outside the SRAM (`Core::aot_slow`); a delayed flow keeps its delay slots in the block. Bails 29 to 0 per pass, interpreted mid-block steps 876 to 256 | ~300 (best of 6, not A/B'd in the same minute) | ~6.5 | x1.32 |
| 1. One handler per transfer shape (commit `perf(sharc): AOT generated code shares one out-of-line handler`) | `hN(core, pc, imm)`: 3 637 sites in 404 handlers; module code ~2.5 MB to ~0.4 MB; build 87 s to 53 s | 251.5 | 5.46 | x1.69 |
| 2. In-place compute (`Pe<'a>` borrows the registers, flags and ASTAT bits of the core instead of copying them) | kernel 123 to 74 cycles per op | 210.9 | 4.58 | x2.05 |
| 2, fast-f32 module (`ORG2F`) | same code, approximate kernels | 184.4 | 4.00 | |

Per core, final build, three cores concurrently (4 runs, mean us, x of 46.08 us): PNO Core1 interpreter 68.5 to 70.1 (x1.49 to x1.52); PNO Core2 interpreter 307 to 352 (x6.67 to x7.65); ORG Core2 AOT 214 to 221 (x4.65 to x4.79); tick (slowest core plus barrier) 311 to 355 us; total CPU per pass 592 to 642 us = 12.8 to 13.9 budget periods. ORG Core2: 6 280 steps in 211 us = 29.8 Msteps/s (18.9 at 333 us); real time needs 136 Msteps/s.

### Why steps 1 and 2 were built this way and registers in locals were not

- The brief named "per-site folded code". The measurement says the opposite layout wins: a pass runs each site once, so per-site inlined code pays instruction fetch and decode for every step, and the merged prototype lost more to code size than folding constants gained. Sharing one handler per shape (immediate as argument) keeps the folded body but fetches it once per shape.
- "Registers in locals" needs a straight-line function that holds the register file across many sites. With shared out-of-line handlers every call is a boundary, so the state lives in memory (the core or a stack struct) anyway, and the layout that could keep registers in machine registers is the large inlined one that lost. The part of the idea that did pay is the element copy per compute op, removed by working in place (step 2). A full locals design was therefore not built.
- Native `DO`, `CALL` and `RTS` were not built: after step 0 no block bails (0 per pass) and the interpreter-run steps are 4.1% of the steps; the remaining flow cost is in dispatch (4.0%) and loop glue (5.8%).

### Where the time goes now (ORG Core2, exact, `--profile` of the final build; shares of pass time, the profile adds overhead)

| Cause | Steps or ops per pass | Share | Note |
|---|---:|---:|---|
| Compute fields on generated `Pe` kernels | 2 147 ops | 17.1% | 74 cycles per op (was 123): the exact 40-bit arithmetic, 13 ns per element in the micro-benchmark |
| Compute fields through `exec_compute_op` | 842 ops | 13.2% | 146 cycles per op: op families `Pe` does not model (shifter, fixed multiplier, other float ops) |
| Lowered transfers without compute | ~4 000 steps | ~35% | transfer handlers, DAG and SRAM access, glue (remainder of the native share) |
| Generic `aot_step` inside blocks | 480 | 9.8% | `Move:Other(4b)` 3.0%, `DirectMem` 2.7%, conditional compute |
| Rest of a block run by the interpreter after a refused entry | 256 | 8.5% | entry refused: hardware loop end inside a block (24 per pass), interrupt event due inside a block (14) |
| Loop glue and idle check | | 5.8% | |
| Block entry and exit | 273 blocks | 4.0% | search, guard, code check |

### Checkpoint and verdict

After the first two steps ORG Core2 is at x4.58, above the x3.5 line, and each step bought less: -25% (shared handlers), -16% (in-place compute), 0% (bail-outs). The compute floor alone is about 2 989 ops x 2 elements x 13 ns = 78 us (x1.7) when the pass runs in SIMD, before any transfer, flow or dispatch work, and every other row above is 4 to 35% of the pass. Even with every fallback row cut to a quarter and compute at the micro-benchmark speed, the pass would stay near x3 in exact mode. **x2 over budget is not reachable in exact mode with this design; stopped as the checkpoint prescribes.** The hybrid verdict (HLE for live play, LLE for oracle and offline work, ROADMAP Gate 1b) is unchanged: ORG Core2 moved from x7.2 to x4.6, but it is still 4.6 times too slow on its own core, and PNO Core2 (x6.7 on the interpreter) would need the same AOT module before it benefits.

What would still change the picture (not done here): fast-f32 as the live tier needs a listening or null test and a cheaper non-compute path (it is x4.0 now); an AOT module for PNO Core2 (its `--aot-pcs` run is not part of this change); modelling the 842 ops per pass that still run on `exec_compute_op` in `Pe` (about -9% by the table above).


## Lowering v2 stages (checkpoint after stage A, slice 1)

Design: [sharc-lowering-v2](sharc-lowering-v2.md). Protocol: section 8 there (no cargo/rustc running, 7 interleaved runs per binary after warm-up, medians with min and max; Ryzen 9 5900XT, desktop, no throttling seen). Tags: EMPIRICAL.

### What was built

Stage A as written (a new IR tree `aot2/`, decode of every executed form, no `aot_step`) was not built in full. The measurements below bound what it can return, so this slice did the cheapest part of its coverage goal and measured it: the fixed-point ALU opcodes `0x0A 0x21 0x22 0x29 0x40..0x42 0x61..0x63` now run on the in-place `Pe` kernel instead of `exec_compute_op` (plus the PE swap). `pe_ops_equal_interpreter_ops_on_random_simd_state` covers them (Exact and Guarded), and `AOT DIFF OK: 273 blocks, boot 100000 steps + 300 passes (1854395 steps) matched 1:1` holds.

### Measurement

| Build | Median us | Min | Max | x budget |
|---|---:|---:|---:|---:|
| step 2 (before this slice), 7 clean runs | 209.4 | 205.9 | 213.8 | 4.54 |
| step 3 (fixed ALU on `Pe`), 7 clean runs | 205.3 | 202.8 | 214.5 | 4.46 |

The medians differ by 2%, less than the larger min-max band (about 4%): **within noise**, so no gain is claimed. Ops through `exec_compute_op` per pass went 842 to 697 (the profile shows 139 cycles per op there against 63 on `Pe`).

### Why this is the stop point

Upper bound of every remaining coverage row of the profile, taking each as free (an impossible best case):

| Row | Share of pass |
|---|---:|
| 697 ops still on `exec_compute_op`, moved to `Pe` speed (139 to 63 cycles) | about 5% |
| 480 generic `aot_step` steps, as cheap as lowered transfers | about 8% |
| 256 steps after a refused entry (hardware loop end, interrupt due) | about 8% |
| block entry/exit and loop glue | about 9% |

Even removing all of it at once gives roughly -30%, from x4.5 to about x3.2. The compute floor in exact mode (about 2 990 ops x 2 elements x 13 ns) is x1.7 alone before any transfer. Stage A's own success line was at most x3.7 and stage B's at most x2.1; the data say stage A ends near x3.5 to x4 and that locals (stage B) address the 35% transfer row, which the earlier experiment showed cannot be paid for with inlined code (2.5 MB lost to 0.4 MB). The curve flattens far above x2: **stopped at the checkpoint as the brief prescribes.** Stages B to D were not started.

### What would still move the number (not done)

- Fast tier as the live tier (x4.0 today; compute is not the bottleneck there), with a listening or null test.
- An exact fast path for the float add and multiply kernels (another session owns `core/aot_fast.rs`).
- A real stage B/C prototype only if someone accepts a result near x2.5 to x3 as worth weeks of work; nothing measured here shows x2.

## Alternatives (what to do instead)

- **HLE hybrid** (recommended by the numbers): run LLE only for cores that fit the budget and keep the Rust HLE engines for the rest. On these measurements only PNO Core1 (x1.6 over, closest) could ever fit; ORG/PNO Core2 are x7 to x8 over.
- **Slower-than-real-time LLE** for non-live uses: preset rendering and null tests of the HLE engines against the firmware.
- **Bounded redesign experiment** (done, section above): shared handlers, in-place compute and fewer bail-outs took ORG Core2 from x7.2 to x4.6 and stopped at the checkpoint; x2 is not reachable in exact mode with this design.

## Implications

- Gate 2 beans (`ns4-lle4` and later) must not assume real-time SHARC execution. Measured gap on the slowest benchmarked core: x8.0 with AOT, x10 to 13 interpreted. All benchmarked cores together need 17 to 20 host cores of budget.
- `pass_sched` and `lle_sharc_bench` are reusable for `ns4-lle4`/`ns4-lle5` and for the follow-up experiment.
- D118 (whole-pass AOT) is answered: bit-exact and buildable, not real time.

## Follow-ups

- Presets and message scripts for ORG Core1 and FX Core1/Core2; SY1SY2/SMP as ISR-driven jobs in `pass_sched`, so the 300-pass benchmark covers all five chips.
- AOT module for PNO Core2 (needs its own `--aot-pcs` run), then re-measure the 3-core tick; `Pe` support for the 842 ops per pass that still use `exec_compute_op`.
- Fast-vs-Exact error in dB on a real pass for the current module build (`sharc_jit_diff --aot-audio` with the approximate module compiled first).
- Listening/null test for the fast-f32 tier, only if the compute share of the pass becomes native.

## Reproduce

```bash
# images and message scripts: see org-system-run.md (local-only)
# per-step rounds: regenerate the modules, rebuild, run the 1:1 diff and the benchmark, then take the minimum mean of 10
# `lle_sharc_bench --engine aot --core ORGc2=...,ORG2` runs per binary, alternating binaries
cargo run --release -p nord-sharc --bin sharc_jit_diff -- IMG ARGS --aot-pcs target/lle/org2.pcs
cargo run --release -p nord-sharc --bin sharc-aot -- IMG --whole-pass target/lle/org2.pcs --force 0x125DDF --tier pe --out target/lle/org2_whole.rs
export NORD_SHARC_AOT_HOT="ORG2=$PWD/target/lle/org2_whole.rs;ORG2F=$PWD/target/lle/org2_whole_approx.rs"
CARGO_TARGET_DIR=target/aot cargo build --release -p nord-tools --bin lle_sharc_bench
target/aot/release/lle_sharc_bench --engine aot --passes 300 \
  --core PNOc1=target/sram/PNO/core1_sram.bin,pno \
  --core PNOc2=target/sram/PNO/core2_sram.bin,org \
  --core ORGc2=target/sram/ORG/core2_sram.bin,org,target/lle/org2.msgtokens,ORG2
```
