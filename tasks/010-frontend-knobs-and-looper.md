# TASK-010: Frontend Rotary Knobs & Looper Transport

- **Ticket ID**: `TASK-010`
- **Status**: `TODO`
- **Dependencies**: `TASK-007`, `TASK-008`
- **Target Files**:
  - `src/components/KnobRack.tsx`
  - `src/components/RotaryKnob.tsx`
  - `src/components/LooperBar.tsx`

---

## 1. Objective

Build the interactive rotary macro knobs and the 2-track looper transport bar:
1. **8 Rotary Knobs (2x4 Grid)**: Authentic brushed aluminum dials with red indicator lines that rotate smoothly when twisted on the MPK mini 3 or dragged with a mouse.
2. **Looper Bar**: Transport buttons (Record, Play, Stop, Clear), track switcher, bar progress tracker, and tempo display.

---

## 2. Implementation Steps

1. **Rotary Knob Component (`RotaryKnob.tsx`)**:
   - Skew-free circular dial rendered with CSS gradient & aluminum texture.
   - Rotational angle constrained between -135° and +135° (270° total sweep).
   - Mouse vertical drag gesture to increment/decrement value smoothly.
   - Real-time synchronization when MIDI CC 70–77 messages arrive from the hardware.
   - Parameter name label (e.g. Cutoff, Resonance, Reverb) + formatted value readout (e.g. `2.4 kHz`, `32%`).

2. **Knob Rack Grid (`KnobRack.tsx`)**:
   - Layout 8 knobs in 2 rows of 4.
   - Header with CC indicators and MIDI Learn button.

3. **Looper Transport Bar (`LooperBar.tsx`)**:
   - Track 1 & Track 2 selection tabs.
   - Circular glowing Record button (red), Play button (emerald), and Clear button.
   - Bar progress indicator (e.g., Beat 1..4 of Bar 1..4).
   - Tempo BPM control with tap-tempo capability.
   - WAV Export button with file download notification.

---

## 3. Acceptance Criteria

- [ ] Turning knob K1 on the MPK mini 3 turns virtual Knob 1 on screen in real time.
- [ ] Dragging Knob 1 with mouse sends updated parameter values to the Rust DSP chain.
- [ ] Looper controls clearly show Recording, Overdubbing, and Playing states.
