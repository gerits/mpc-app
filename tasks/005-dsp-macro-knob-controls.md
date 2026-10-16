# TASK-005: Macro DSP Controls & CC Mapping Chain

- **Ticket ID**: `TASK-005`
- **Status**: `DONE`
- **Dependencies**: `TASK-002`, `TASK-003`
- **Target Files**:
  - `src-tauri/src/dsp.rs`
  - `src-tauri/src/effects/reverb.rs`
  - `src-tauri/src/effects/filter.rs`
  - `src-tauri/src/effects/chorus.rs`

---

## 1. Objective

Implement a lightweight, low-latency DSP effects chain in Rust that processes the audio output in real time based on the Akai MPK mini 3's 8 rotary knobs (K1–K8). Provide smooth parameter interpolation (zipper-noise free) and MIDI CC mapping.

---

## 2. Implementation Steps

1. **8 Macro Parameters**:
   - **K1: Filter Cutoff**: 2-pole resonant low-pass filter (20 Hz to 20,000 Hz, logarithmic curve).
   - **K2: Filter Resonance (Q)**: Adjustable resonance peaking from 0.707 (flat) to 8.0.
   - **K3: Attack Envelope Modifier**: Pre-gain or SoundFont CC 73 envelope attack scaling.
   - **K4: Release Envelope Modifier**: SoundFont CC 72 release time scaling.
   - **K5: Reverb Mix**: Freeverb / Schroeder stereo reverberator with wet/dry crossfade.
   - **K6: Chorus / Delay Mix**: Stereo modulated delay with feedback and width.
   - **K7: Master Output Gain**: -inf to +6 dB clean gain.
   - **K8: Keys vs Drums Balance**: Crossfades relative gain between melodic synth and drum synth.

2. **DSP Filter & Effects Processing (`dsp.rs`)**:
   - State-variable filter (SVF) or Biquad filter implementation with smoothed coefficients.
   - Stereo Freeverb implementation tuned for ambient warmth without high CPU load.
   - Chorus delay buffer with sine LFO modulation.

3. **Parameter Smoothing**:
   - One-pole lowpass filter smoothing on knob changes to prevent clicks or zipper noise when turning knobs quickly.

---

## 3. Acceptance Criteria

- [x] Turning K1 audibly sweeps the filter cutoff frequency without popping or clicking.
- [x] Turning K5 introduces smooth stereo reverb.
- [x] Turning K8 shifts the balance cleanly between melodic keys and drums.
- [x] DSP processing adds < 0.5ms of latency to the audio callback.
