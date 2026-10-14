# TASK-003: Pure-Rust Dual SoundFont Synthesis & Audio Stream

- **Ticket ID**: `TASK-003`
- **Status**: `TODO`
- **Dependencies**: `TASK-001`, `TASK-002`
- **Target Files**:
  - `src-tauri/src/audio.rs`
  - `src-tauri/src/synth.rs`
  - `src-tauri/Cargo.toml` (add `cpal`, `rustysynth` or `oxisynth`)

---

## 1. Objective

Set up the low-latency audio output thread using `cpal` and integrate a pure Rust SoundFont synthesizer engine (`rustysynth`). Implement a dual-synth architecture where one synth instance handles melodic keyboard instruments and a second independent synth instance handles the drum kit, avoiding soundfont program conflict and allowing independent soundfont swapping and mixing.

---

## 2. Implementation Steps

1. **CPAL Audio Stream Setup (`audio.rs`)**:
   - Initialize the default host and output device (CoreAudio on macOS).
   - Configure a low-latency stream buffer (e.g., 128 or 256 frames, 44.1kHz or 48kHz).
   - Create a lock-free render callback receiving stereo audio frames (`&mut [f32]`).

2. **Dual Synth Architecture (`synth.rs`)**:
   - Maintain two `Synthesizer` instances:
     - `melodic_synth`: Renders keyboard note-on, note-off, pitch bend, and modulation.
     - `drum_synth`: Renders pad hits on General MIDI standard percussion mapping (Channel 10, Bank 128 or dedicated drum kit SF2).
   - In each audio callback block:
     - Pull pending MIDI routed events from the lock-free queue.
     - Dispatch events to the corresponding synthesizer.
     - Render left/right audio buffers from each synth and sum them into the master stereo buffer with gain control.

3. **Peak Meter Calculation**:
   - Compute RMS / peak amplitude on rendered audio blocks to broadcast to the UI meter at ~30–60 FPS.

---

## 3. Acceptance Criteria

- [ ] Audio stream starts cleanly without buffer underruns (xruns) or audible clicking.
- [ ] Keyboard events trigger sounds through `melodic_synth`.
- [ ] Pad events trigger percussion hits through `drum_synth` simultaneously without cutting off decaying melodic notes.
- [ ] Changing soundfonts on the melodic synth does not interrupt drum playback.
