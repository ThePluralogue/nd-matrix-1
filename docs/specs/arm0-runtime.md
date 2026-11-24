# ARM0 runtime (`crates/nord-arm`)

Status: v0.1 (2026-11-23), bean `ns4-lle2`, decision [D244](../DECISIONS.md). Measurements: [arm0-rust-runtime](../research/arm0-rust-runtime.md).

Native Rust ARM926EJ-S (ARMv5TE) interpreter that boots the user's OS v1.66 ARM0 image with the hardware the Python harness stubbed. Opt-in crate (no dependencies, not linked by the audio engine). Firmware bytes are never committed: the tests use synthetic in-memory code.

## Layout

| Module | Role |
|---|---|
| `cpu/` | `Cpu`, ARM and Thumb decode (`arm*.rs`, `thumb*.rs`), `alu.rs`, `Bus` trait. WFI (`mcr p15,0,rd,c7,c0,4`) returns `Fault::Wfi`. |
| `mem.rs` | 4 GiB sparse paged memory, per-page flags `HOOK_R/W/X`, unmapped access maps a zero page. |
| `machine/mod.rs` | `Machine::run(max, until, stop_pc)`, `call_preserving`, hooks table, null-call guard. |
| `machine/hooks.rs` | `Hook` enum: `Return`, `BumpNew`, `BoardConfig`, `SpiTransfer`, `SkipInsn`, `ZeroR5`, `BoardPresent`, `DspRead`. |
| `machine/boot.rs` | Firmware load at `0xC0000000`, boot stubs, init array, `run_tick`, timer IRQ 22 injection. |
| `machine/sysbus.rs` | Tick counter `0x80001DC0`, GPIO echo, poll breaker, fake DSP ports (path 1 `0xA0011000`, path 2 `0xA0010000`), port write log. |
| `machine/{panel,i2c,uart}.rs` | Front panel on CS2, four 64 KB I2C EEPROMs (program memory), UART2 16550 MIDI input. |
| `machine/system.rs` | `boot_system`, `send_midi`, `drain_messages` (ARM to DSP messages from the port log). |
| `machine/dsp_return.rs` | DSP to ARM SPI reads (`0xC00119A4`, `0xC00117E8`, `0xC0066E64`): SY idle voice mask, then shadow of the ARM's own messages, then 0. |
| `machine/idle.rs` | Idle skipping. |

## Time model

One tick = 1 ms virtual = 200,000 instructions (ASSUMED) followed by one timer IRQ. Interrupts are not CPU exceptions: the AINTC handler `IRQ_TABLE[irq]` is called with all registers and the CPSR preserved (sp lowered by `0x800`, lr = `SENTINEL`), as in the Python harness. The low 64 KB is zeroed between ticks and null calls return 0.

## Idle skipping (exact)

From state `S0`, a probe runs up to `MAX_PERIOD = 4096` instructions on an instrumented bus. If the CPU returns to the same registers/CPSR and, meanwhile, no store changed memory, no device page (tick, poll windows, GPIO, DSP ports) was touched, no hook ran and no unmapped page was mapped, the state equals `S0`. Then `k * L` instructions are added to the virtual counter without executing and the remainder runs for real. `WFI` skips the rest of the tick. Probes start after 64 executed instructions and back off to 8192. Verified by `arm-lle-bench`: state hash equal to full execution after every phase.

## Acceptance (`ns4-lle2`)

| Item | Status |
|---|---|
| Opt-in native core, choice recorded | done, D244 |
| Stubs ported, no Python callbacks | done (timer, ports, SPI read return partial: `ns4-lle5` for live layers) |
| Idle / WFI / poll fast-forward | done, exact |
| < 1.0 ms/tick on real boot + load + keys | mean yes (0.21 to 0.41 ms), 2.5 to 4.8 % of ticks over 1 ms |
| Synthetic tests only | done |
