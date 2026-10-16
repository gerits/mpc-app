# TASK-007: Tauri IPC State, Commands & Realtime Events

- **Ticket ID**: `TASK-007`
- **Status**: `DONE`
- **Dependencies**: `TASK-001`, `TASK-002`, `TASK-003`, `TASK-005`, `TASK-006`
- **Target Files**:
  - `src-tauri/src/commands.rs`
  - `src-tauri/src/state.rs`
  - `src-tauri/src/main.rs`
  - `src/types/ipc.ts`

---

## 1. Objective

Connect the Rust audio/MIDI backend with the React frontend through Tauri v2's thread-safe state management, invokable commands, and low-overhead event channels.

---

## 2. Implementation Steps

1. **Shared Application State (`state.rs`)**:
   - `AppState`: Thread-safe wrappers (`Arc<Mutex<...>>` or channels) for:
     - Active MIDI controller status and port list.
     - Melodic instrument soundfont & preset index.
     - Drum kit soundfont & preset index.
     - DSP macro knob values (array of 8 floats).
     - Looper state (Track 1 & 2 state, current bar, BPM).
     - Master output volume and RMS peak meter.

2. **Tauri Commands (`commands.rs`)**:
   - `get_audio_devices() -> Vec<AudioDevice>`
   - `get_midi_devices() -> Vec<MidiDevice>`
   - `set_midi_device(device_name: String) -> Result<(), String>`
   - `get_soundfonts() -> SoundFontCatalog`
   - `select_melodic_soundfont(id: String, preset: u32) -> Result<(), String>`
   - `select_drum_soundfont(id: String) -> Result<(), String>`
   - `set_macro_parameter(index: usize, value: f32) -> Result<(), String>`
   - `looper_record(track: usize, bars: u32) -> Result<(), String>`
   - `looper_play(track: usize) -> Result<(), String>`
   - `looper_clear(track: usize) -> Result<(), String>`
   - `looper_export(track: Option<usize>) -> Result<String, String>`

3. **Realtime Event Streaming**:
   - Emit `midi-event`: Dispatched whenever a pad is struck or knob turned on hardware, so the UI can illuminate pads and rotate virtual knobs in exact synchronization.
   - Emit `audio-meter`: Dispatched at 30 FPS with left/right RMS peak levels.

4. **TypeScript Definitions (`ipc.ts`)**:
   - Strongly-typed helper wrappers matching all Rust structs and event payloads.

---

## 3. Acceptance Criteria

- [x] Frontend can call all Tauri commands and receive valid typed responses.
- [x] Hardware pad hit on the MPK mini 3 emits an event to the frontend in <2ms.
- [x] Rotating hardware knob K1 emits `midi-event` updating frontend knob state without lag.
