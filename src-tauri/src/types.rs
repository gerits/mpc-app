use serde::{Deserialize, Serialize};

/// Represents information about an enumerated MIDI input device.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MidiDeviceInfo {
    pub index: usize,
    pub name: String,
    pub is_akai: bool,
}

/// A recorded musical note in a loop track.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LoopNote {
    pub id: String,
    pub track_index: usize, // 0 = Drums, 1 = Keys
    pub note: u8,           // MIDI note number (36..51 for pads, 0..127 for keys)
    pub velocity: u8,       // 1..=127
    pub start_step: f32,    // Position in 16th-note steps (e.g. 0.0 .. 32.0 for 2 bars)
    pub length_steps: f32,  // Duration in 16th-note steps
    pub off_beat: bool,     // True if not aligned to nearest 1/16th grid within tolerance
}

/// Raw parsed MIDI Channel and System events.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum MidiEvent {
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
    PitchBend {
        channel: u8,
        value: i16, // -8192..=8191
    },
    ProgramChange {
        channel: u8,
        program: u8,
    },
    ChannelAftertouch {
        channel: u8,
        pressure: u8,
    },
    Other {
        status: u8,
        data: Vec<u8>,
    },
}

impl MidiEvent {
    /// Parse raw MIDI byte slices into structured `MidiEvent`.
    /// Note that `NoteOn` with velocity `0` is standardized as `NoteOff`.
    pub fn parse(bytes: &[u8]) -> Option<Self> {
        if bytes.is_empty() {
            return None;
        }

        let status = bytes[0];
        let message_type = status & 0xF0;
        let channel = status & 0x0F;

        match message_type {
            0x80 => {
                if bytes.len() >= 3 {
                    Some(MidiEvent::NoteOff {
                        channel,
                        note: bytes[1],
                        velocity: bytes[2],
                    })
                } else if bytes.len() == 2 {
                    Some(MidiEvent::NoteOff {
                        channel,
                        note: bytes[1],
                        velocity: 0,
                    })
                } else {
                    None
                }
            }
            0x90 => {
                if bytes.len() >= 3 {
                    let note = bytes[1];
                    let velocity = bytes[2];
                    if velocity == 0 {
                        Some(MidiEvent::NoteOff {
                            channel,
                            note,
                            velocity: 0,
                        })
                    } else {
                        Some(MidiEvent::NoteOn {
                            channel,
                            note,
                            velocity,
                        })
                    }
                } else {
                    None
                }
            }
            0xB0 => {
                if bytes.len() >= 3 {
                    Some(MidiEvent::ControlChange {
                        channel,
                        controller: bytes[1],
                        value: bytes[2],
                    })
                } else {
                    None
                }
            }
            0xC0 => {
                if bytes.len() >= 2 {
                    Some(MidiEvent::ProgramChange {
                        channel,
                        program: bytes[1],
                    })
                } else {
                    None
                }
            }
            0xD0 => {
                if bytes.len() >= 2 {
                    Some(MidiEvent::ChannelAftertouch {
                        channel,
                        pressure: bytes[1],
                    })
                } else {
                    None
                }
            }
            0xE0 => {
                if bytes.len() >= 3 {
                    let lsb = bytes[1] as u16;
                    let msb = bytes[2] as u16;
                    let raw = (msb << 7) | lsb;
                    let centered = (raw as i32 - 8192) as i16;
                    Some(MidiEvent::PitchBend {
                        channel,
                        value: centered,
                    })
                } else {
                    None
                }
            }
            _ => Some(MidiEvent::Other {
                status,
                data: bytes[1..].to_vec(),
            }),
        }
    }
}

/// Zero-config routed events separated into distinct performance streams:
/// keyboard notes, MPC rubber drum pads, macro rotary knobs, joystick modulation & pitch bend.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum RoutedEvent {
    KeyboardNoteOn {
        channel: u8,
        note: u8,
        velocity: u8,
    },
    KeyboardNoteOff {
        channel: u8,
        note: u8,
    },
    DrumPadOn {
        pad_index: u8, // 0..=15 (Pads 1-8 on Bank A, 9-16 on Bank B)
        note: u8,
        velocity: u8,
    },
    DrumPadOff {
        pad_index: u8,
        note: u8,
    },
    MacroKnob {
        knob_index: u8, // 0..=7 (mapped from CC 70..=77)
        value: f32,     // 0.0 ..= 1.0 normalized
    },
    Modulation {
        value: f32,     // 0.0 ..= 1.0 normalized (from CC 1 / Joystick Y)
    },
    PitchBend {
        value: f32,     // -1.0 ..= 1.0 normalized (from Pitch Bend / Joystick X)
    },
    SustainPedal {
        is_down: bool,  // CC 64
    },
    Raw(MidiEvent),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_midi_event_parsing() {
        assert_eq!(MidiEvent::parse(&[]), None);

        // Note On
        assert_eq!(
            MidiEvent::parse(&[0x90, 60, 100]),
            Some(MidiEvent::NoteOn {
                channel: 0,
                note: 60,
                velocity: 100
            })
        );

        // Note On with velocity 0 -> parsed as Note Off
        assert_eq!(
            MidiEvent::parse(&[0x90, 60, 0]),
            Some(MidiEvent::NoteOff {
                channel: 0,
                note: 60,
                velocity: 0
            })
        );

        // Note Off
        assert_eq!(
            MidiEvent::parse(&[0x81, 60, 64]),
            Some(MidiEvent::NoteOff {
                channel: 1,
                note: 60,
                velocity: 64
            })
        );

        // Control Change
        assert_eq!(
            MidiEvent::parse(&[0xB0, 74, 127]),
            Some(MidiEvent::ControlChange {
                channel: 0,
                controller: 74,
                value: 127
            })
        );

        // Pitch Bend
        assert_eq!(
            MidiEvent::parse(&[0xE0, 0x00, 0x40]),
            Some(MidiEvent::PitchBend {
                channel: 0,
                value: 0
            })
        );

        // Program Change
        assert_eq!(
            MidiEvent::parse(&[0xC2, 5]),
            Some(MidiEvent::ProgramChange {
                channel: 2,
                program: 5
            })
        );
    }
}
