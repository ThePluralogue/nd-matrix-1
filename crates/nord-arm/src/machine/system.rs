//! Full-board boot: panel, I2C EEPROMs, UART2 MIDI (`tools/arm_screens.py::Session`,
//! `tools/arm_keydown.py`, `tools/arm_panel.py::boot_with_panels`).

use super::boot::{APP_THUNK, CHUNK, MAIN_LOOP, TIMER_IRQ};
use super::idle::IdleStats;
use super::panel::{KeyMatrix, WAIT_LOOP};
use super::uart::{Uart2, BOARD_ATTACH, UART2_IRQ};
use super::{Hook, Machine, Stop, SENTINEL, STACK_TOP};

impl Machine {
    /// `arm_screens.Session.__init__` models: key matrix, 4 EEPROMs (blank `0xFF`), UART2, and the
    /// board-present hook. `prepare` must have run with the UI stubs removed first, see
    /// [`Machine::prepare_system`].
    pub fn attach_board(&mut self) {
        self.bus.panel = Some(KeyMatrix::default());
        self.bus.i2c = Some(super::i2c::I2cBus::new());
        self.bus.uart2 = Some(Uart2::default());
        self.add_hook(BOARD_ATTACH, Hook::BoardPresent);
    }

    /// `prepare(display_stubs=False)` + board models: the display objects run for real.
    pub fn prepare_system(&mut self, code: &[u8], sram: &[u8]) -> Result<(), Stop> {
        self.load_firmware(code, sram);
        self.install_boot_stubs();
        // the panel link needs the display routines unstubbed (arm_boot_full.install_ui_stubs)
        self.hooks.remove(&0xC001_B67C);
        self.hooks.remove(&0xC001_B9D0);
        self.run_init_array()?;
        self.attach_board();
        Ok(())
    }

    /// Boot to the main loop, serving the wait loop with timer interrupts (idle skipping on).
    /// Returns `(instructions, interrupts injected, stop)`.
    pub fn boot_system(&mut self, max: u64, stats: &mut IdleStats) -> (u64, u64, Stop) {
        self.cpu.r[13] = STACK_TOP - 0x1000;
        self.cpu.r[14] = SENTINEL;
        self.cpu.thumb = false;
        self.cpu.r[15] = APP_THUNK;
        self.stop_pc = Some(MAIN_LOOP);
        let start = self.cpu.insns;
        let mut irqs = 0;
        let stop = loop {
            if self.cpu.insns - start >= max {
                break Stop::Budget;
            }
            let s = self.run_idle_skip(CHUNK, stats);
            self.bus.mem.zero(0, super::LOW_MEMORY as usize);
            if !matches!(s, Stop::Budget) {
                break s;
            }
            let pc = self.cpu.r[15];
            if (WAIT_LOOP.0..=WAIT_LOOP.1).contains(&pc) {
                self.inject_irq(TIMER_IRQ);
                irqs += 1;
            }
        };
        self.stop_pc = None;
        (self.cpu.insns - start, irqs, stop)
    }

    /// Queue MIDI bytes on UART2 and run its receive ISR (`Uart2.push`).
    pub fn send_midi(&mut self, data: &[u8]) {
        if let Some(u) = &mut self.bus.uart2 {
            u.push(data);
        }
        self.inject_irq(UART2_IRQ);
    }

    /// Take the DSP messages logged since the last call, grouped (`arm_emu.group_port_writes`).
    pub fn drain_messages(&mut self) -> Vec<Message> {
        if let Some(r) = &mut self.dsp_return {
            r.sync(&self.bus.port_writes);
            r.reset_seen();
        }
        let w = std::mem::take(&mut self.bus.port_writes);
        group_port_writes(&w)
    }
}

/// One ARM to DSP message after grouping the port stores.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Message {
    /// 1 or 2 (see `docs/research/arm-dsp-protocol.md`).
    pub path: u8,
    pub comm: u8,
    pub word1: u32,
    pub word2: u32,
    pub pc: u32,
}

/// Group `(port, value, pc)` stores: path 2 ports carry `(address, [pad], value)`, path 1 ports
/// `(word1, word2)`; incomplete trailing groups are dropped (`arm_emu.group_port_writes`).
pub fn group_port_writes(writes: &[(u32, u32, u32)]) -> Vec<Message> {
    use super::sysbus::{COMM_COUNT, PORT1_BASE, PORT2_BASE};
    let mut pending: std::collections::HashMap<u32, Vec<(u32, u32)>> = Default::default();
    let mut out = Vec::new();
    for &(port, value, pc) in writes {
        let (path, comm, need) = if (PORT2_BASE..PORT2_BASE + 0x10 * COMM_COUNT).contains(&port) {
            let c = (port - PORT2_BASE) / 0x10;
            (
                2u8,
                c,
                if super::boot::PAD[c as usize] != 0 {
                    3
                } else {
                    2
                },
            )
        } else if (PORT1_BASE..PORT1_BASE + 0x10 * COMM_COUNT).contains(&port) {
            (1u8, (port - PORT1_BASE) / 0x10, 2)
        } else {
            continue;
        };
        let buf = pending.entry(port).or_default();
        buf.push((value, pc));
        if buf.len() == need {
            out.push(Message {
                path,
                comm: comm as u8,
                word1: buf[0].0,
                word2: buf[need - 1].0,
                pc: buf[need - 1].1,
            });
            buf.clear();
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::machine::sysbus::{PORT1_BASE, PORT2_BASE};

    #[test]
    fn grouping_follows_pad_and_path() {
        let w = [
            (PORT2_BASE + 0x10, 0x2800_0010, 1), // comm 1 (padded): addr, pad, value
            (PORT2_BASE + 0x10, 0, 2),
            (PORT2_BASE + 0x10, 0x3F80_0000, 3),
            (PORT1_BASE + 0x30, 0x92414, 4), // comm 3 path 1: two words
            (PORT1_BASE + 0x30, 0x1C0794, 5),
            (PORT2_BASE, 0x2800_0000, 6), // incomplete
        ];
        let m = group_port_writes(&w);
        assert_eq!(m.len(), 2);
        assert_eq!(
            (m[0].path, m[0].comm, m[0].word1, m[0].word2),
            (2, 1, 0x2800_0010, 0x3F80_0000)
        );
        assert_eq!(
            (m[1].path, m[1].comm, m[1].word2, m[1].pc),
            (1, 3, 0x1C0794, 5)
        );
    }

    #[test]
    fn attach_board_registers_models_and_hook() {
        let mut m = Machine::new();
        m.attach_board();
        assert!(m.bus.panel.is_some() && m.bus.i2c.is_some() && m.bus.uart2.is_some());
        m.cpu.r[0] = 0x2000;
        m.cpu.r[15] = BOARD_ATTACH;
        m.bus.mem.write32(BOARD_ATTACH, 0xE1A0_0000);
        m.run(1, 0, None);
        assert_eq!(m.bus.mem.read8(0x2012), 1);
        assert_eq!(m.bus.mem.read8(0x2013), 1);
    }

    #[test]
    fn send_midi_queues_bytes_even_without_an_isr() {
        let mut m = Machine::new();
        m.attach_board();
        m.send_midi(&[0x90, 60, 100]);
        assert_eq!(m.bus.uart2.as_ref().unwrap().queue.len(), 3);
    }
}
