use crate::types::{MidiEvent, RoutedEvent};

/// Configuration options for the MIDI router.
#[derive(Debug, Clone)]
pub struct RouterConfig {
    /// Starting note for drum pads (default 36, C1 on Akai MPK mini Bank A Pad 1)
    pub pad_start_note: u8,
    /// Ending note for drum pads (default 51, D#2 on Akai MPK mini Bank B Pad 8)
    pub pad_end_note: u8,
    /// 0-indexed MIDI channel dedicated to drums (default 9 = MIDI Channel 10)
    pub drum_channel: u8,
    /// Starting CC number for macro knobs (default 70 for Akai MPK mini K1)
    pub knob_start_cc: u8,
    /// Number of macro knobs (default 8)
    pub knob_count: u8,
}

impl Default for RouterConfig {
    fn default() -> Self {
        Self {
            pad_start_note: 36, // C1
            pad_end_note: 51,   // D#2
            drum_channel: 9,    // MIDI Channel 10
            knob_start_cc: 70,  // CC 70 = K1
            knob_count: 8,      // 8 Knobs (CC 70 - 77)
        }
    }
}

/// Zero-Config MIDI Event Router.
/// Automatically splits incoming MIDI byte streams into distinct performance streams:
/// - Drum pads: Notes 36..=51 or MIDI channel 10
/// - Melodic keyboard: Notes outside drum pad range
/// - Macro knobs: CC 70..=77 (Akai MPK mini K1..K8)
/// - Joystick: Pitch bend and Modulation (CC 1)
#[derive(Debug, Clone, Default)]
pub struct MidiRouter {
    pub config: RouterConfig,
}

impl MidiRouter {
    pub fn new() -> Self {
        Self {
            config: RouterConfig::default(),
        }
    }

    pub fn with_config(config: RouterConfig) -> Self {
        Self { config }
    }

    /// Parse raw MIDI byte slices and route into a structured `RoutedEvent`.
    pub fn route_raw(&self, bytes: &[u8]) -> Option<RoutedEvent> {
        let midi_event = MidiEvent::parse(bytes)?;
        Some(self.route(&midi_event))
    }

    /// Route a parsed `MidiEvent` based on the Akai MPK mini separation rules.
    pub fn route(&self, event: &MidiEvent) -> RoutedEvent {
        match event {
            MidiEvent::NoteOn {
                channel,
                note,
                velocity,
            } => {
                if self.is_drum_event(*channel, *note) {
                    let pad_index = self.calculate_pad_index(*channel, *note);
                    RoutedEvent::DrumPadOn {
                        pad_index,
                        note: *note,
                        velocity: *velocity,
                    }
                } else {
                    RoutedEvent::KeyboardNoteOn {
                        channel: *channel,
                        note: *note,
                        velocity: *velocity,
                    }
                }
            }
            MidiEvent::NoteOff {
                channel,
                note,
                velocity: _,
            } => {
                if self.is_drum_event(*channel, *note) {
                    let pad_index = self.calculate_pad_index(*channel, *note);
                    RoutedEvent::DrumPadOff {
                        pad_index,
                        note: *note,
                    }
                } else {
                    RoutedEvent::KeyboardNoteOff {
                        channel: *channel,
                        note: *note,
                    }
                }
            }
            MidiEvent::ControlChange {
                channel: _,
                controller,
                value,
            } => {
                let end_cc = self.config.knob_start_cc.saturating_add(self.config.knob_count);
                if *controller >= self.config.knob_start_cc && *controller < end_cc {
                    let knob_index = controller - self.config.knob_start_cc;
                    let normalized_val = (*value as f32) / 127.0;
                    RoutedEvent::MacroKnob {
                        knob_index,
                        value: normalized_val.clamp(0.0, 1.0),
                    }
                } else if *controller == 1 {
                    // CC 1: Modulation wheel / Joystick vertical axis
                    let normalized_val = (*value as f32) / 127.0;
                    RoutedEvent::Modulation {
                        value: normalized_val.clamp(0.0, 1.0),
                    }
                } else if *controller == 64 {
                    // CC 64: Sustain Pedal
                    RoutedEvent::SustainPedal {
                        is_down: *value >= 64,
                    }
                } else {
                    RoutedEvent::Raw(event.clone())
                }
            }
            MidiEvent::PitchBend { channel: _, value } => {
                // value is -8192..=8191
                let normalized = (*value as f32) / 8192.0;
                RoutedEvent::PitchBend {
                    value: normalized.clamp(-1.0, 1.0),
                }
            }
            _ => RoutedEvent::Raw(event.clone()),
        }
    }

    /// Determines if an event should be routed to the drum pad sound engine.
    #[inline]
    fn is_drum_event(&self, channel: u8, note: u8) -> bool {
        channel == self.config.drum_channel
            || (note >= self.config.pad_start_note && note <= self.config.pad_end_note)
    }

    /// Calculates a pad index in range 0..16.
    #[inline]
    fn calculate_pad_index(&self, channel: u8, note: u8) -> u8 {
        if note >= self.config.pad_start_note && note <= self.config.pad_end_note {
            note - self.config.pad_start_note
        } else if channel == self.config.drum_channel {
            if note < self.config.pad_start_note {
                note % 16
            } else {
                (note - self.config.pad_start_note) % 16
            }
        } else {
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::RoutedEvent;

    #[test]
    fn test_note_routing_drum_pad_vs_keyboard() {
        let router = MidiRouter::new();

        // Note 36 (C1) on Channel 0 -> DrumPad 0
        let note36_on = [0x90, 36, 100];
        let routed = router.route_raw(&note36_on).expect("routed event");
        assert_eq!(
            routed,
            RoutedEvent::DrumPadOn {
                pad_index: 0,
                note: 36,
                velocity: 100,
            }
        );

        // Note 36 Note Off
        let note36_off = [0x80, 36, 64];
        let routed_off = router.route_raw(&note36_off).expect("routed event");
        assert_eq!(
            routed_off,
            RoutedEvent::DrumPadOff {
                pad_index: 0,
                note: 36,
            }
        );

        // Note 51 (D#2) -> DrumPad 15 (Bank B Pad 8)
        let note51_on = [0x90, 51, 127];
        let routed_pad15 = router.route_raw(&note51_on).expect("routed event");
        assert_eq!(
            routed_pad15,
            RoutedEvent::DrumPadOn {
                pad_index: 15,
                note: 51,
                velocity: 127,
            }
        );

        // Note 60 (Middle C) on Channel 0 -> KeyboardNoteOn
        let note60_on = [0x90, 60, 80];
        let routed_key = router.route_raw(&note60_on).expect("routed event");
        assert_eq!(
            routed_key,
            RoutedEvent::KeyboardNoteOn {
                channel: 0,
                note: 60,
                velocity: 80,
            }
        );

        // Note 60 Note Off via 0x80
        let note60_off = [0x80, 60, 0];
        let routed_key_off = router.route_raw(&note60_off).expect("routed event");
        assert_eq!(
            routed_key_off,
            RoutedEvent::KeyboardNoteOff {
                channel: 0,
                note: 60,
            }
        );

        // NoteOn with velocity 0 treated as NoteOff
        let note60_vel0 = [0x90, 60, 0];
        let routed_vel0 = router.route_raw(&note60_vel0).expect("routed event");
        assert_eq!(
            routed_vel0,
            RoutedEvent::KeyboardNoteOff {
                channel: 0,
                note: 60,
            }
        );
    }

    #[test]
    fn test_drum_channel_10_routing() {
        let router = MidiRouter::new();

        // Channel 9 (MIDI Channel 10), Note 60 -> Routed to DrumPad
        let ch10_note = [0x99, 60, 110];
        let routed = router.route_raw(&ch10_note).expect("routed event");
        match routed {
            RoutedEvent::DrumPadOn { pad_index, note, velocity } => {
                assert_eq!(note, 60);
                assert_eq!(velocity, 110);
                assert!(pad_index < 16);
            }
            _ => panic!("Expected DrumPadOn for Channel 10 note"),
        }
    }

    #[test]
    fn test_macro_knobs_cc70_to_77() {
        let router = MidiRouter::new();

        for i in 0..8u8 {
            let cc = 70 + i;
            let val = 127;
            let msg = [0xB0, cc, val];
            let routed = router.route_raw(&msg).expect("routed event");
            assert_eq!(
                routed,
                RoutedEvent::MacroKnob {
                    knob_index: i,
                    value: 1.0,
                }
            );

            // Half value (64 / 127 = 0.5039...)
            let msg_half = [0xB0, cc, 64];
            let routed_half = router.route_raw(&msg_half).expect("routed event");
            match routed_half {
                RoutedEvent::MacroKnob { knob_index, value } => {
                    assert_eq!(knob_index, i);
                    assert!((value - (64.0 / 127.0)).abs() < 1e-4);
                }
                _ => panic!("Expected MacroKnob"),
            }

            // Zero value
            let msg_zero = [0xB0, cc, 0];
            let routed_zero = router.route_raw(&msg_zero).expect("routed event");
            assert_eq!(
                routed_zero,
                RoutedEvent::MacroKnob {
                    knob_index: i,
                    value: 0.0,
                }
            );
        }
    }

    #[test]
    fn test_joystick_modulation_and_pitch_bend() {
        let router = MidiRouter::new();

        // CC 1 (Modulation Wheel)
        let mod_msg = [0xB0, 1, 127];
        let routed_mod = router.route_raw(&mod_msg).expect("routed event");
        assert_eq!(routed_mod, RoutedEvent::Modulation { value: 1.0 });

        // Pitch Bend Center (0x2000 = 8192 -> 0.0)
        let pb_center = [0xE0, 0x00, 0x40];
        let routed_pb = router.route_raw(&pb_center).expect("routed event");
        assert_eq!(routed_pb, RoutedEvent::PitchBend { value: 0.0 });

        // Pitch Bend Max (0x3FFF = 16383 -> ~1.0)
        let pb_max = [0xE0, 0x7F, 0x7F];
        let routed_pb_max = router.route_raw(&pb_max).expect("routed event");
        match routed_pb_max {
            RoutedEvent::PitchBend { value } => {
                assert!((value - (8191.0 / 8192.0)).abs() < 1e-4);
            }
            _ => panic!("Expected PitchBend"),
        }

        // Pitch Bend Min (0x0000 -> -1.0)
        let pb_min = [0xE0, 0x00, 0x00];
        let routed_pb_min = router.route_raw(&pb_min).expect("routed event");
        assert_eq!(routed_pb_min, RoutedEvent::PitchBend { value: -1.0 });
    }

    #[test]
    fn test_sustain_pedal() {
        let router = MidiRouter::new();

        let pedal_down = [0xB0, 64, 127];
        let routed = router.route_raw(&pedal_down).expect("routed event");
        assert_eq!(routed, RoutedEvent::SustainPedal { is_down: true });

        let pedal_up = [0xB0, 64, 0];
        let routed_up = router.route_raw(&pedal_up).expect("routed event");
        assert_eq!(routed_up, RoutedEvent::SustainPedal { is_down: false });
    }
}
