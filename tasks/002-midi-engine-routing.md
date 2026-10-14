# TASK-002: Native MIDI Device Discovery & Auto-Routing

- **Ticket ID**: `TASK-002`
- **Status**: `DONE`
- **Dependencies**: `TASK-001`
- **Target Files**:
  - `src-tauri/src/midi.rs`
  - `src-tauri/src/router.rs`
  - `src-tauri/src/types.rs`
  - `src-tauri/Cargo.toml` (add `midir`, `crossbeam-channel`)

---

## 1. Objective

Implement low-latency native MIDI input listening via `midir` (CoreMIDI on macOS, ALSA on Linux, WinMM/WinRT on Windows). Detect the Akai MPK mini 3 upon connection and automatically route incoming MIDI messages into distinct streams:
1. **Keyboard Notes**: All musical notes outside the pad range (or Channel 1).
2. **Drum Pad Hits**: Notes 36 through 51 (C1 to D#2) or any note on MIDI Channel 10.
3. **Macro Knobs**: CC messages (CC 70–77 or customizable range).
4. **Joystick**: Pitch bend wheel and Modulation wheel (CC 1).

---

## 2. Implementation Steps

1. **MIDI Device Enumeration**:
   - Query all available MIDI input ports via `midir::MidiInput`.
   - Auto-match port names containing `"MPK mini"`, `"MPK"`, or fallback to the first active MIDI controller.
   - Listen for hotplug events or provide a periodic re-scan function.

2. **MIDI Message Parsing (`router.rs`)**:
   - Parse raw MIDI bytes into structured enums:
     - `MidiEvent::NoteOn { channel, note, velocity }`
     - `MidiEvent::NoteOff { channel, note }`
     - `MidiEvent::ControlChange { controller, value }`
     - `MidiEvent::PitchBend { value }`
   - Implement the **Zero-Config Separation Rule**:
     - If note is in range `36..=51` or `channel == 9` (0-indexed Channel 10), classify as `RoutedEvent::DrumPad { pad_index: note - 36, velocity }`.
     - Otherwise, classify as `RoutedEvent::KeyboardNote { note, velocity }`.
     - CC 70..=77 classify as `RoutedEvent::MacroKnob { knob_index: cc - 70, value }`.
     - CC 1 or Pitch Bend classify as `RoutedEvent::Modulation`.

3. **Event Queue**:
   - Send parsed routed events over a lock-free `crossbeam-channel` or atomic queue to the audio synth thread and UI event emitter.

---

## 3. Acceptance Criteria

- [x] Unit tests in `router.rs` verify that Note 36 (C1) routes to `DrumPad 0` and Note 60 (Middle C) routes to `KeyboardNote`.
- [x] CC 70-77 are accurately translated into 0-7 knob indices with normalized 0.0–1.0 float values.
- [x] Pluggable MIDI listener connects to connected hardware controller without dropping note-off events.

