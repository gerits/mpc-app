# TASK-006: 2-Track Quick Looper & WAV Exporter

- **Ticket ID**: `TASK-006`
- **Status**: `TODO`
- **Dependencies**: `TASK-003`, `TASK-005`
- **Target Files**:
  - `src-tauri/src/looper.rs`
  - `src-tauri/src/wav_export.rs`
  - `src-tauri/Cargo.toml` (add `hound` for WAV encoding)

---

## 1. Objective

Build a built-in 2-track audio looper engine. This allows the musician to record a drum groove on Track 1, set it to loop continuously in sync with a master tempo (BPM), and overdub or jam live with the melodic keyboard on Track 2. Include one-click export of recorded loops to 24-bit WAV files.

---

## 2. Implementation Steps

1. **Looper Engine Architecture (`looper.rs`)**:
   - Track structure containing circular stereo sample buffers:
     - `Track 1`: Typically used for drum grooves or foundation loops.
     - `Track 2`: Melodic loop or overdub layer.
   - States: `Empty`, `Recording`, `Playing`, `Overdubbing`, `Muted`.
   - Tempo & Bar Quantization:
     - Configurable BPM (e.g. default 120.0 BPM).
     - Auto-calculates exact sample count for 1, 2, 4, or 8 bars to ensure seamless loop wrap-around without clicks.

2. **Real-time Loop Playback & Recording**:
   - Embedded directly in the audio callback.
   - While `Recording`: Writes summed or source-isolated audio samples into the circular buffer.
   - While `Playing`: Sums loop buffer samples into the master output.
   - While `Overdubbing`: Blends incoming audio with existing buffer samples at specified decay ratio.

3. **WAV File Export (`wav_export.rs`)**:
   - Write recorded buffers into standard 44.1kHz/48kHz 24-bit stereo WAV using `hound`.
   - Save directly to user's Music / Documents folder with timestamped filenames (`MiniStation_Loop_20261014_120BPM.wav`).

---

## 3. Acceptance Criteria

- [ ] Hitting Record captures audio for exact 4 bars and transitions seamlessly into loop playback.
- [ ] Overdubbing adds key notes on top of an existing drum loop without phase cancellation or clipping.
- [ ] Exporting produces a clean WAV file playable in any media player.
