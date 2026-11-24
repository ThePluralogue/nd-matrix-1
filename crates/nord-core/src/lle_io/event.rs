//! Engine-neutral live input events for the LLE backend (bean `ns4-lle6`).
//!
//! One `Copy` enum covers live MIDI (re-using [`crate::midi::MidiEvent`] fields), pedals and panel
//! edits. It packs into 16 bytes ([`Wire`]) so it travels through the wait-free ring unchanged.
//! The future `LleEngine` turns each event into ARM0 input-queue traffic; nothing here knows the
//! ARM0 side.

use super::ring::Wire;
use crate::midi::{cc, MidiEvent};

/// Foot pedal (or pedal-like controller) the firmware treats separately from plain CCs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pedal {
    /// CC 64.
    Sustain,
    /// CC 66.
    Sostenuto,
    /// CC 67.
    Soft,
    /// Continuous control pedal, CC 11 (Nord "Control Pedal" jack).
    Control,
}

impl Pedal {
    /// Pedal a MIDI controller number stands for, if any.
    pub fn from_cc(controller: u8) -> Option<Self> {
        match controller {
            cc::SUSTAIN_PEDAL => Some(Self::Sustain),
            cc::SOSTENUTO_PEDAL => Some(Self::Sostenuto),
            cc::SOFT_PEDAL => Some(Self::Soft),
            cc::CONTROL_PEDAL => Some(Self::Control),
            _ => None,
        }
    }

    fn tag(self) -> u8 {
        self as u8
    }

    fn from_tag(t: u8) -> Self {
        match t {
            1 => Self::Sostenuto,
            2 => Self::Soft,
            3 => Self::Control,
            _ => Self::Sustain,
        }
    }
}

/// Front-panel edit, addressed by the firmware's own ids.
///
/// VERIFIED: encoder `index` and key `id` are the ids the ARM0 scan decoder posts as UI events
/// (`{tag, source, encoder index, delta}` and `{tag, 0, key id, 1 / 0}`), source
/// `tools/arm_events.py` module doc, `docs/research/arm-oled-render.md` s15 (Program dial = index
/// 6, PIANO SELECT = 7, Synth dials = 15, 14, 13); checked by `tools/arm_events.py turn`, not by
/// a Rust test.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelEvent {
    /// Endless dial turned by `delta` detents (negative = counter-clockwise).
    Encoder { index: u8, delta: i16 },
    /// Button or key `id` pressed (`true`) or released.
    Key { id: u8, pressed: bool },
    /// Analog control (knob, fader, drawbar, wheel, pitch stick) at `value`, `0..=65535` of its
    /// travel. ASSUMED: the firmware's ADC scale is applied later by the engine wiring
    /// (`ns4-lle5`); full-scale u16 keeps this layer scale-free.
    Analog { id: u16, value: u16 },
}

/// One live input event for the LLE engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LleEvent {
    /// Empty slot (also what an unknown wire tag decodes to).
    #[default]
    Nop,
    NoteOn {
        channel: u8,
        note: u8,
        velocity: u8,
    },
    NoteOff {
        channel: u8,
        note: u8,
        velocity: u8,
    },
    ControlChange {
        channel: u8,
        controller: u8,
        value: u8,
    },
    ChannelAftertouch {
        channel: u8,
        pressure: u8,
    },
    /// 14-bit pitch bend, `0..=16383`, centre `8192`.
    PitchBend {
        channel: u8,
        value: u16,
    },
    ProgramChange {
        channel: u8,
        program: u8,
    },
    Pedal {
        channel: u8,
        pedal: Pedal,
        value: u8,
    },
    Panel(PanelEvent),
}

impl From<MidiEvent> for LleEvent {
    /// Pedal controllers (64, 66, 67, 11) become [`LleEvent::Pedal`]; every other CC stays a CC.
    fn from(e: MidiEvent) -> Self {
        match e {
            MidiEvent::NoteOn {
                channel,
                note,
                velocity,
            } => Self::NoteOn {
                channel,
                note,
                velocity,
            },
            MidiEvent::NoteOff {
                channel,
                note,
                velocity,
            } => Self::NoteOff {
                channel,
                note,
                velocity,
            },
            MidiEvent::ControlChange {
                channel,
                controller,
                value,
            } => match Pedal::from_cc(controller) {
                Some(pedal) => Self::Pedal {
                    channel,
                    pedal,
                    value,
                },
                None => Self::ControlChange {
                    channel,
                    controller,
                    value,
                },
            },
            MidiEvent::ChannelAftertouch { channel, pressure } => {
                Self::ChannelAftertouch { channel, pressure }
            }
            MidiEvent::ProgramChange { channel, program } => {
                Self::ProgramChange { channel, program }
            }
            MidiEvent::PitchBend { channel, value } => Self::PitchBend { channel, value },
        }
    }
}

impl Wire for LleEvent {
    fn to_wire(self) -> [u8; 16] {
        let mut b = [0u8; 16];
        match self {
            Self::Nop => {}
            Self::NoteOn {
                channel,
                note,
                velocity,
            } => b[..4].copy_from_slice(&[1, channel, note, velocity]),
            Self::NoteOff {
                channel,
                note,
                velocity,
            } => b[..4].copy_from_slice(&[2, channel, note, velocity]),
            Self::ControlChange {
                channel,
                controller,
                value,
            } => b[..4].copy_from_slice(&[3, channel, controller, value]),
            Self::ChannelAftertouch { channel, pressure } => {
                b[..3].copy_from_slice(&[4, channel, pressure]);
            }
            Self::PitchBend { channel, value } => {
                b[..2].copy_from_slice(&[5, channel]);
                b[2..4].copy_from_slice(&value.to_le_bytes());
            }
            Self::ProgramChange { channel, program } => {
                b[..3].copy_from_slice(&[6, channel, program]);
            }
            Self::Pedal {
                channel,
                pedal,
                value,
            } => b[..4].copy_from_slice(&[7, channel, pedal.tag(), value]),
            Self::Panel(PanelEvent::Encoder { index, delta }) => {
                b[..2].copy_from_slice(&[8, index]);
                b[2..4].copy_from_slice(&delta.to_le_bytes());
            }
            Self::Panel(PanelEvent::Key { id, pressed }) => {
                b[..3].copy_from_slice(&[9, id, u8::from(pressed)]);
            }
            Self::Panel(PanelEvent::Analog { id, value }) => {
                b[0] = 10;
                b[1..3].copy_from_slice(&id.to_le_bytes());
                b[3..5].copy_from_slice(&value.to_le_bytes());
            }
        }
        b
    }

    fn from_wire(b: [u8; 16]) -> Self {
        let u16_at = |i: usize| u16::from_le_bytes([b[i], b[i + 1]]);
        match b[0] {
            1 => Self::NoteOn {
                channel: b[1],
                note: b[2],
                velocity: b[3],
            },
            2 => Self::NoteOff {
                channel: b[1],
                note: b[2],
                velocity: b[3],
            },
            3 => Self::ControlChange {
                channel: b[1],
                controller: b[2],
                value: b[3],
            },
            4 => Self::ChannelAftertouch {
                channel: b[1],
                pressure: b[2],
            },
            5 => Self::PitchBend {
                channel: b[1],
                value: u16_at(2),
            },
            6 => Self::ProgramChange {
                channel: b[1],
                program: b[2],
            },
            7 => Self::Pedal {
                channel: b[1],
                pedal: Pedal::from_tag(b[2]),
                value: b[3],
            },
            8 => Self::Panel(PanelEvent::Encoder {
                index: b[1],
                delta: u16_at(2) as i16,
            }),
            9 => Self::Panel(PanelEvent::Key {
                id: b[1],
                pressed: b[2] != 0,
            }),
            10 => Self::Panel(PanelEvent::Analog {
                id: u16_at(1),
                value: u16_at(3),
            }),
            _ => Self::Nop,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all_variants() -> Vec<LleEvent> {
        vec![
            LleEvent::Nop,
            LleEvent::NoteOn {
                channel: 15,
                note: 127,
                velocity: 1,
            },
            LleEvent::NoteOff {
                channel: 0,
                note: 21,
                velocity: 64,
            },
            LleEvent::ControlChange {
                channel: 3,
                controller: cc::MOD_WHEEL,
                value: 99,
            },
            LleEvent::ChannelAftertouch {
                channel: 2,
                pressure: 77,
            },
            LleEvent::PitchBend {
                channel: 1,
                value: 16383,
            },
            LleEvent::ProgramChange {
                channel: 9,
                program: 5,
            },
            LleEvent::Pedal {
                channel: 0,
                pedal: Pedal::Sostenuto,
                value: 127,
            },
            LleEvent::Panel(PanelEvent::Encoder {
                index: 6,
                delta: -3,
            }),
            LleEvent::Panel(PanelEvent::Key {
                id: 21,
                pressed: true,
            }),
            LleEvent::Panel(PanelEvent::Analog {
                id: 513,
                value: 65535,
            }),
        ]
    }

    #[test]
    fn every_variant_round_trips_through_the_wire_form() {
        for e in all_variants() {
            assert_eq!(LleEvent::from_wire(e.to_wire()), e, "{e:?}");
        }
    }

    #[test]
    fn zeroed_slot_and_unknown_tag_decode_to_nop() {
        assert_eq!(LleEvent::from_wire([0; 16]), LleEvent::Nop);
        let mut b = [0u8; 16];
        b[0] = 200;
        assert_eq!(LleEvent::from_wire(b), LleEvent::Nop);
    }

    #[test]
    fn all_pedals_round_trip() {
        for p in [
            Pedal::Sustain,
            Pedal::Sostenuto,
            Pedal::Soft,
            Pedal::Control,
        ] {
            let e = LleEvent::Pedal {
                channel: 4,
                pedal: p,
                value: 10,
            };
            assert_eq!(LleEvent::from_wire(e.to_wire()), e);
        }
    }

    #[test]
    fn midi_pedal_ccs_become_pedal_events_and_others_stay_ccs() {
        let cc_ev = |controller| MidiEvent::ControlChange {
            channel: 2,
            controller,
            value: 100,
        };
        for (c, p) in [
            (cc::SUSTAIN_PEDAL, Pedal::Sustain),
            (cc::SOSTENUTO_PEDAL, Pedal::Sostenuto),
            (cc::SOFT_PEDAL, Pedal::Soft),
            (cc::CONTROL_PEDAL, Pedal::Control),
        ] {
            assert_eq!(
                LleEvent::from(cc_ev(c)),
                LleEvent::Pedal {
                    channel: 2,
                    pedal: p,
                    value: 100
                }
            );
        }
        assert_eq!(
            LleEvent::from(cc_ev(cc::MOD_WHEEL)),
            LleEvent::ControlChange {
                channel: 2,
                controller: cc::MOD_WHEEL,
                value: 100
            }
        );
    }

    #[test]
    fn every_midi_event_kind_converts() {
        let n = LleEvent::from(MidiEvent::NoteOn {
            channel: 1,
            note: 60,
            velocity: 90,
        });
        assert!(matches!(n, LleEvent::NoteOn { note: 60, .. }));
        assert_eq!(
            LleEvent::from(MidiEvent::PitchBend {
                channel: 0,
                value: 8192
            }),
            LleEvent::PitchBend {
                channel: 0,
                value: 8192
            }
        );
        assert_eq!(
            LleEvent::from(MidiEvent::ProgramChange {
                channel: 0,
                program: 7
            }),
            LleEvent::ProgramChange {
                channel: 0,
                program: 7
            }
        );
        assert_eq!(
            LleEvent::from(MidiEvent::ChannelAftertouch {
                channel: 0,
                pressure: 5
            }),
            LleEvent::ChannelAftertouch {
                channel: 0,
                pressure: 5
            }
        );
        assert!(matches!(
            LleEvent::from(MidiEvent::NoteOff {
                channel: 0,
                note: 1,
                velocity: 0
            }),
            LleEvent::NoteOff { note: 1, .. }
        ));
    }
}
