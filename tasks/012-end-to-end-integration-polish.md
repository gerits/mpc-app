# TASK-012: End-to-End Latency Verification, Build & Documentation

- **Ticket ID**: `TASK-012`
- **Status**: `TODO`
- **Dependencies**: All preceding tickets (`TASK-001` through `TASK-011`)
- **Target Files**:
  - `README.md`
  - `LICENSE`
  - Integration smoke tests

---

## 1. Objective

Perform complete end-to-end testing with an Akai MPK mini 3 controller, verify that round-trip audio latency is sub-10ms, produce release distribution builds (`.dmg` for macOS, portable binary), and write user/developer documentation.

---

## 2. Implementation Steps

1. **End-to-End Smoke Testing**:
   - Plug in Akai MPK mini 3: verify auto-detection and port assignment.
   - Play 25 keys: verify polyphony, velocity sensitivity, pitch bend, and modulation joystick response.
   - Hit pads: verify instantaneous drum kit playback without interrupting sustained keyboard notes.
   - Turn knobs: verify smooth real-time filter sweeps and reverb without CPU spikes or zipper noise.
   - Record 4-bar loop with pads, overdub chords with keys, export to WAV, and verify exported audio file integrity.

2. **Performance & Latency Profiling**:
   - Verify CPU usage is < 5% on idle and < 15% under heavy polyphonic playing.
   - Verify total memory footprint is < 60 MB RAM.

3. **Packaging & Documentation**:
   - Create comprehensive `README.md` with:
     - Overview, screenshots/diagrams.
     - Quick start guide for MPK mini 3 users.
     - Build instructions (`bun run tauri build`).
     - Attribution for all included open-source SoundFonts.
   - Include open-source `LICENSE` (MIT or Apache 2.0).

---

## 3. Acceptance Criteria

- [ ] Complete workflow works flawlessly with no errors in the console.
- [ ] Release bundle builds successfully into standalone macOS app / DMG.
- [ ] `README.md` clearly guides users through plugging in their MPK mini 3 and playing immediately.
