//! DSP to ARM return channel: the three SPI read routines answered natively
//! (`tools/arm_dsp_return.py`).
//!
//! Layers, first hit wins: SY idle voice mask, the DM shadow of every word the ARM itself sent,
//! then 0. The live-chip and initial-image layers of the Python module belong to `ns4-lle5`.

use super::boot::COMM_TABLE;
use super::sysbus::{COMM_COUNT, PORT1_BASE, PORT2_BASE};
use super::system::Message;
use super::Machine;
use std::collections::HashMap;

/// VERIFIED (disassembly, `docs/research/dsp-to-arm-return.md` s2): entry points, argument order.
pub const SPI_READ4: u32 = 0xC001_19A4;
pub const SPI_READ2: u32 = 0xC001_17E8;
pub const SPI_READN: u32 = 0xC006_6E64;
const SY_CHIP: u32 = 3;
/// ASSUMED cap: the decoded sites read 1 to 4 words.
const READN_MAX: u32 = 4;
/// VERIFIED (oracle run, `tools/arm_dsp_return.py`): SY idle voice mask words, 56 voices.
const IDLE_MASK: u32 = 0x92196;
const RELEASE_MASK: u32 = 0x9219A;
const VOICES: u32 = 56;
const STATE_BASE: u32 = 0x92414;
const STATE_STRIDE: u32 = 0x38;
const IDLE_STATES: [u32; 2] = [0x1C07B0, 0x1C0794];
const BUSY_STATES: [u32; 2] = [0x1C07CD, 0x1C084C];

/// Return source state.
#[derive(Clone, Debug)]
pub struct DspReturn {
    pub shadow: HashMap<(u32, u32, u32), u32>,
    pub sy_idle: u64,
    pub reads: u64,
    pending: HashMap<u32, Vec<(u32, u32)>>,
    seen: usize,
}

impl Default for DspReturn {
    fn default() -> Self {
        Self {
            shadow: HashMap::new(),
            sy_idle: (1u64 << VOICES) - 1,
            reads: 0,
            pending: HashMap::new(),
            seen: 0,
        }
    }
}

/// `(core, DM word)` of a DSP address as the ARM sends it (PNO/ORG alias, bit 23 = core 2).
pub fn resolve(chip: u32, address: u32) -> (u32, u32) {
    if (0x2800_0000..0x2C00_0000).contains(&address) && (chip == 1 || chip == 2) {
        return (
            if address & 0x80_0000 != 0 { 2 } else { 1 },
            (address & 0x7F_FFFF) >> 2,
        );
    }
    (0, address)
}

impl DspReturn {
    fn observe(&mut self, m: &Message) {
        let chip = m.comm as u32;
        if m.path == 2 {
            let (core, dm) = resolve(chip, m.word1);
            self.shadow.insert((chip, core, dm), m.word2);
        } else {
            self.shadow.insert((chip, 0, m.word1), m.word2);
            let off = m.word1.wrapping_sub(STATE_BASE);
            if chip == SY_CHIP && off.is_multiple_of(STATE_STRIDE) && off / STATE_STRIDE < VOICES {
                let bit = 1u64 << (off / STATE_STRIDE);
                if IDLE_STATES.contains(&m.word2) {
                    self.sy_idle |= bit;
                } else if BUSY_STATES.contains(&m.word2) {
                    self.sy_idle &= !bit;
                }
            }
        }
    }

    /// Feed the port stores logged since the last call (grouping persists across calls).
    pub fn sync(&mut self, writes: &[(u32, u32, u32)]) {
        for &(port, value, pc) in &writes[self.seen.min(writes.len())..] {
            let (path, comm, need) = if (PORT2_BASE..PORT2_BASE + 0x10 * COMM_COUNT).contains(&port)
            {
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
            let buf = self.pending.entry(port).or_default();
            buf.push((value, pc));
            if buf.len() == need {
                let msg = Message {
                    path,
                    comm: comm as u8,
                    word1: buf[0].0,
                    word2: buf[need - 1].0,
                    pc,
                };
                buf.clear();
                self.observe(&msg);
            }
        }
        self.seen = writes.len();
    }

    /// Called when the machine drains its port log.
    pub fn reset_seen(&mut self) {
        self.seen = 0;
    }

    fn mask_word(&self, dm: u32) -> Option<u32> {
        if (IDLE_MASK..IDLE_MASK + 2).contains(&dm) {
            Some((self.sy_idle >> (32 * (dm - IDLE_MASK))) as u32)
        } else if (RELEASE_MASK..RELEASE_MASK + 2).contains(&dm) {
            Some(0)
        } else {
            None
        }
    }

    fn word(&self, chip: u32, core: u32, dm: u32) -> u32 {
        if chip == SY_CHIP {
            if let Some(w) = self.mask_word(dm) {
                return w;
            }
        }
        self.shadow.get(&(chip, core, dm)).copied().unwrap_or(0)
    }

    pub fn knows(&self, chip: u32, address: u32) -> bool {
        let (core, dm) = resolve(chip, address);
        (chip == SY_CHIP && self.mask_word(dm).is_some())
            || self.shadow.contains_key(&(chip, core, dm))
    }

    pub fn read(&mut self, chip: u32, address: u32, count: u32) -> Vec<u32> {
        self.reads += 1;
        let (core, dm) = resolve(chip, address);
        (0..count).map(|k| self.word(chip, core, dm + k)).collect()
    }
}

impl Machine {
    /// Replace the three SPI read routines by answers from [`DspReturn`].
    pub fn install_dsp_return(&mut self) {
        self.dsp_return = Some(DspReturn::default());
        for pc in [SPI_READ4, SPI_READ2, SPI_READN] {
            self.add_hook(pc, super::Hook::DspRead);
        }
    }

    /// Run the read routine at `pc` in place of the firmware code.
    pub(crate) fn dsp_read(&mut self, pc: u32) {
        let mut src = self.dsp_return.take().expect("installed");
        src.sync(&self.bus.port_writes);
        let r = self.cpu.r;
        let (chip, address, count, dst) = if pc == SPI_READN {
            let chip = (0..5u32)
                .find(|c| self.bus.mem.read32(COMM_TABLE + 4 + 4 * c) == r[0])
                .unwrap_or(u32::MAX);
            let dst = self.bus.mem.read32(r[13]);
            (chip, r[2], r[3], dst)
        } else {
            (r[1], r[2], if pc == SPI_READ4 { 4 } else { 2 }, r[3])
        };
        let ok = pc != SPI_READN
            || (chip < 5 && count > 0 && count <= READN_MAX && src.knows(chip, address));
        if ok {
            for (k, w) in src.read(chip, address, count).into_iter().enumerate() {
                self.bus.mem.write32(dst + 4 * k as u32, w);
            }
        } else if pc == SPI_READN && dst != 0 {
            self.bus.mem.write32(dst, 0);
        }
        self.dsp_return = Some(src);
        self.cpu.r[0] = 0;
        self.cpu.r[15] = self.cpu.r[14] & !1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_maps_the_alias_to_core_and_word() {
        assert_eq!(resolve(1, 0x2800_0010), (1, 4));
        assert_eq!(resolve(2, 0x2880_0010), (2, 4));
        assert_eq!(resolve(3, 0x92196), (0, 0x92196));
    }

    #[test]
    fn shadow_and_sy_mask_follow_the_arm_messages() {
        let mut r = DspReturn::default();
        // PNO (comm 1, padded): address, pad, value; SY (comm 3) path 1 state message of voice 1
        let w = [
            (PORT2_BASE + 0x10, 0x2800_0010, 1),
            (PORT2_BASE + 0x10, 0, 1),
            (PORT2_BASE + 0x10, 0xAA55, 1),
            (PORT1_BASE + 0x30, STATE_BASE + STATE_STRIDE, 2),
            (PORT1_BASE + 0x30, 0x1C07CD, 2),
        ];
        r.sync(&w);
        assert_eq!(r.read(1, 0x2800_0010, 1), vec![0xAA55]);
        assert!(r.knows(1, 0x2800_0010) && !r.knows(1, 0x2800_0014));
        assert_eq!(
            r.read(SY_CHIP, IDLE_MASK, 2),
            vec![0xFFFF_FFFD, 0x00FF_FFFF]
        );
        r.sync(&w);
        assert_eq!(r.shadow.len(), 2);
    }

    #[test]
    fn read4_stub_returns_to_the_caller_and_counts() {
        let mut m = Machine::new();
        m.install_dsp_return();
        m.cpu.r = [0; 16];
        m.cpu.r[1] = 1;
        m.cpu.r[2] = 0x2800_0000;
        m.cpu.r[3] = 0x2000;
        m.cpu.r[14] = 0xC000_0100;
        m.cpu.r[15] = SPI_READ4;
        m.run(1, 0xC000_0100, None);
        assert_eq!(m.cpu.r[15], 0xC000_0100);
        assert_eq!(m.bus.mem.read32(0x200C), 0);
        assert_eq!(m.dsp_return.as_ref().unwrap().reads, 1);
    }
}
