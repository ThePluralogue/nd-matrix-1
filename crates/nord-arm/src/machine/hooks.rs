//! Code hooks: function stubs and one-instruction patches of the Python harness, as enum arms.

use super::{Machine, LOW_MEMORY};
use crate::cpu::Bus;

/// A hook runs when the instruction at its address is about to execute.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hook {
    /// Replace the function: `r0 = value`, then return to the caller (`pc = lr & !1`, the state
    /// bit is left as is, like `arm_emu.ArmEmu._stub_hook`).
    Return(u32),
    /// `operator new` bump allocator over zero pages (`ArmEmu.bump_new`).
    BumpNew,
    /// Board configuration query: `*r2 = 0`, returns 1 (`arm_boot.install_boot_stubs`).
    BoardConfig,
    /// SPI master transfer: the fifth argument (`[sp]`) receives 0, returns 0
    /// (`arm_boot_full.install_ui_stubs`).
    SpiTransfer,
    /// Make the instruction a no-op (`pc += 4`), e.g. the MMU-enable `mcr`.
    SkipInsn,
    /// Set `r5 = 0` and execute the instruction normally (`SPI_COUNT_CHECK`).
    ZeroR5,
    /// Board probe reports "present": `[r0 + 0x12..0x14] = 1, 1`, then run normally
    /// (`arm_keydown.install_board_present`).
    BoardPresent,
    /// DSP to ARM SPI read routine, answered by `DspReturn`.
    DspRead,
}

impl Machine {
    /// Run the hook at `pc`. Returns true when `pc` was redirected (the caller re-examines it).
    pub(crate) fn run_hook(&mut self, pc: u32) -> bool {
        if pc < LOW_MEMORY && self.null_guard {
            self.null_calls.push((pc, self.cpu.r[14]));
            self.cpu.r[0] = 0;
            self.cpu.r[15] = self.cpu.r[14] & !1;
            return true;
        }
        let Some(h) = self.hooks.get(&pc).copied() else {
            return false;
        };
        match h {
            Hook::Return(v) => self.stub_return(v),
            Hook::BumpNew => {
                let p = self.heap_top;
                self.heap_top = p.wrapping_add((self.cpu.r[0].wrapping_add(15) & !15).max(16));
                self.stub_return(p);
            }
            Hook::BoardConfig => {
                let a = self.cpu.r[2];
                self.bus.write32(a, 0);
                self.stub_return(1);
            }
            Hook::SpiTransfer => {
                let out = self.bus.read32(self.cpu.r[13]);
                if out != 0 {
                    self.bus.write32(out, 0);
                }
                self.stub_return(0);
            }
            Hook::SkipInsn => self.cpu.r[15] = pc.wrapping_add(4),
            Hook::DspRead => self.dsp_read(pc),
            Hook::BoardPresent => {
                let a = self.cpu.r[0] + super::uart::BOARD_PRESENT_OFFSET;
                self.bus.mem.write8(a, 1);
                self.bus.mem.write8(a + 1, 1);
                return false;
            }
            Hook::ZeroR5 => {
                self.cpu.r[5] = 0;
                return false;
            }
        }
        true
    }

    fn stub_return(&mut self, r0: u32) {
        self.cpu.r[0] = r0;
        self.cpu.r[15] = self.cpu.r[14] & !1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::machine::Stop;

    fn m() -> Machine {
        let mut m = Machine::new();
        m.cpu.r[15] = 0xC000_0000;
        m
    }

    #[test]
    fn return_stub_replaces_function_and_sets_r0() {
        let mut m = m();
        // bl 0xC0000010 ; mov r2,#9 ; ... stub at 0xC0000010
        m.bus.mem.write32(0xC000_0000, 0xEB00_0002);
        m.bus.mem.write32(0xC000_0004, 0xE3A0_2009);
        m.add_hook(0xC000_0010, Hook::Return(0x55));
        let (n, stop) = m.run(2, 0, None);
        assert_eq!((n, stop), (2, Stop::Budget));
        assert_eq!((m.reg(0), m.reg(2)), (0x55, 9));
    }

    #[test]
    fn bump_new_hands_out_aligned_blocks() {
        let mut m = m();
        m.heap_top = 0x9000_0000;
        m.add_hook(0xC000_0100, Hook::BumpNew);
        m.cpu.r[14] = 0xC000_0000;
        for expect in [0x9000_0000u32, 0x9000_0010, 0x9000_0030] {
            m.cpu.r[15] = 0xC000_0100;
            m.cpu.r[0] = if expect == 0x9000_0010 { 20 } else { 1 };
            assert!(m.run_hook(0xC000_0100));
            assert_eq!(m.reg(0), expect);
        }
        assert_eq!(m.heap_top, 0x9000_0040);
    }

    #[test]
    fn null_guard_returns_zero_to_the_caller() {
        let mut m = m();
        m.guard_null_calls();
        // mov lr,#0xC0000008 can't be encoded; put lr by hand and jump to 0x20
        m.cpu.r[14] = 0xC000_0008;
        m.cpu.r[15] = 0x20;
        m.cpu.r[0] = 7;
        m.bus.mem.write32(0xC000_0008, 0xE3A0_1001);
        m.run(2, 0, None);
        assert_eq!((m.reg(0), m.reg(1)), (0, 1));
        assert_eq!(m.null_calls, vec![(0x20, 0xC000_0008)]);
    }

    #[test]
    fn skip_zero_r5_and_board_config() {
        let mut m = m();
        m.cpu.r[5] = 9;
        m.add_hook(0xC000_0000, Hook::ZeroR5);
        m.bus.mem.write32(0xC000_0000, 0xE1A0_6005); // mov r6,r5
        m.run(1, 0, None);
        assert_eq!(m.reg(6), 0);
        m.add_hook(0xC000_0004, Hook::SkipInsn);
        m.bus.mem.write32(0xC000_0004, 0xE3A0_7001);
        assert!(m.run_hook(0xC000_0004));
        assert_eq!((m.reg(7), m.reg(15)), (0, 0xC000_0008));
        m.add_hook(0xC000_0008, Hook::BoardConfig);
        m.cpu.r[2] = 0x2000;
        m.bus.mem.write32(0x2000, 5);
        m.cpu.r[14] = 0xC000_0100;
        assert!(m.run_hook(0xC000_0008));
        assert_eq!((m.reg(0), m.bus.mem.read32(0x2000)), (1, 0));
    }
}
