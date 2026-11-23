//! Bus tap of `sharc-live` (D235, OQ41): reads an arbitrary DM bus pair after every audio
//! interrupt instead of the SMP post stage ring.
//!
//! The ring tap of D200 carries only the bus the post stage sums (`0x9404E`, `0x9403E`); the
//! voices of piano layer B write `0x9405A` (D230), which no ring carries. `B ADDR [f]` taps the
//! words `ADDR` (left) and `ADDR + 1` (right) of the bus. A bus word is an integer sample that the
//! post stage converts with `FLOAT BY -29` (VERIFIED, `piano-bus-convention.md` s2b: ring =
//! bus * 2^-29, post stage unity), so the default mode scales the signed word by `2^-29` to a float
//! frame (full-scale stroke `2^30` = 2.0) and the captured file reads like the ring tap; the `f`
//! mode passes a word that already holds float bits through.
//!
//! The tap is off by default (`Option::None` in `SyModel`): the audio interrupt then reads the ring
//! exactly as before, the check is one `Option` test per frame.
use crate::core::Core;

/// Scale of an integer bus word to the float frame of the ring tap.
/// VERIFIED: `piano-bus-convention.md` s2b (post stage `FLOAT BY -29`, unity, 300 interrupt run).
pub const BUS_SCALE: f32 = 1.0 / 536_870_912.0; // 2^-29

/// A bus pair to tap.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BusTap {
    /// Address of the left word; the right word is `addr + 1`.
    pub addr: u32,
    /// The words hold float bits already (no `2^-29` scaling).
    pub float: bool,
}

impl BusTap {
    /// Parses the arguments of `B`: `ADDR [f]`; address 0 switches the tap off (`Ok(None)`).
    pub fn parse(addr: u32, mode: Option<&str>) -> Result<Option<Self>, String> {
        let float = match mode {
            None | Some("i") => false,
            Some("f") => true,
            Some(m) => return Err(format!("B: bad mode {m:?} (want i or f)")),
        };
        Ok((addr != 0).then_some(Self { addr, float }))
    }

    /// One word to float bits.
    pub fn convert(self, word: u32) -> u32 {
        if self.float {
            word
        } else {
            (word as i32 as f32 * BUS_SCALE).to_bits()
        }
    }

    /// The stereo frame `[left, right]` (float bits) the bus holds now.
    pub fn frame(self, core: &mut Core) -> [u32; 2] {
        [
            self.convert(core.dm_read(self.addr).unwrap_or(0)),
            self.convert(core.dm_read(self.addr + 1).unwrap_or(0)),
        ]
    }

    /// The `B` reply.
    pub fn reply(tap: Option<Self>, file: bool) -> String {
        match tap {
            Some(t) => format!(
                "B addr={:x} mode={}{}",
                t.addr,
                if t.float { 'f' } else { 'i' },
                if file { " file" } else { "" }
            ),
            None => "B off".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn core() -> Core {
        Core::new(&vec![0u8; 0x24_0000])
    }

    #[test]
    fn address_zero_is_off_and_modes_parse() {
        assert_eq!(BusTap::parse(0, None), Ok(None));
        assert_eq!(
            BusTap::parse(0x9405A, None),
            Ok(Some(BusTap {
                addr: 0x9405A,
                float: false
            }))
        );
        assert_eq!(
            BusTap::parse(0x9405A, Some("f")),
            Ok(Some(BusTap {
                addr: 0x9405A,
                float: true
            }))
        );
        assert!(BusTap::parse(1, Some("x")).is_err());
    }

    #[test]
    fn integer_words_scale_by_two_to_the_minus_29_and_keep_the_sign() {
        let t = BusTap {
            addr: 1,
            float: false,
        };
        assert_eq!(f32::from_bits(t.convert(1 << 30)), 2.0);
        assert_eq!(f32::from_bits(t.convert(0xE000_0000)), -1.0); // -2^29
        assert_eq!(t.convert(0), 0);
        let f = BusTap {
            addr: 1,
            float: true,
        };
        assert_eq!(f.convert(0x3F80_0000), 0x3F80_0000);
    }

    #[test]
    fn the_frame_reads_the_pair_and_a_missing_word_is_silent() {
        let mut c = core();
        c.dm_write(0x9_405A, 14_015_760).unwrap();
        c.dm_write(0x9_405B, 1 << 29).unwrap();
        let t = BusTap {
            addr: 0x9_405A,
            float: false,
        };
        let [l, r] = t.frame(&mut c);
        assert!(
            (f32::from_bits(l) - 0.026_105).abs() < 1e-5,
            "{}",
            f32::from_bits(l)
        );
        assert_eq!(f32::from_bits(r), 1.0);
        let far = BusTap {
            addr: 0xFFF_FFFF,
            float: false,
        };
        assert_eq!(far.frame(&mut c), [0, 0]);
    }

    #[test]
    fn the_reply_names_the_tap() {
        assert_eq!(BusTap::reply(None, false), "B off");
        let t = BusTap {
            addr: 0x9_405A,
            float: false,
        };
        assert_eq!(BusTap::reply(Some(t), false), "B addr=9405a mode=i");
        assert_eq!(BusTap::reply(Some(t), true), "B addr=9405a mode=i file");
    }
}
