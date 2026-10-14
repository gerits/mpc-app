# TASK-008: Frontend Shell: Akai Classic Studio Chassis

- **Ticket ID**: `TASK-008`
- **Status**: `TODO`
- **Dependencies**: `TASK-001`, `TASK-007`
- **Target Files**:
  - `src/App.tsx`
  - `src/components/Header.tsx`
  - `src/components/Meter.tsx`
  - `src/components/OledScreen.tsx`
  - `src/index.css`

---

## 1. Objective

Build the main application shell using the user-approved **Variation 1: Akai Classic Studio** aesthetic. Implement the signature matte graphite chassis (`#121316`), contrasting red borders, vintage OLED screen styling, and dual real-time stereo LED peak meters.

---

## 2. Implementation Steps

1. **Hardware Chassis Framework (`App.tsx`)**:
   - Fixed desktop aspect layout with authentic chamfered edges, dark carbon textures, and subtle hardware screw accents.
   - Structured 3-row layout:
     - **Top**: Connectivity header, master looper bar, master output volume & stereo LED meters.
     - **Middle**: Melodic keyboard engine (left) + Macro knobs & Drum pads (right).
     - **Bottom**: Status ticker, MIDI activity indicator, and settings trigger.

2. **Top Header & OLED Display (`Header.tsx`, `OledScreen.tsx`)**:
   - Emerald/amber phosphor CRT styling with monospace lettering.
   - Live hardware status showing `"AKAI MPK MINI 3 (PORT 1)"` with pulse dot when connected, or `"NO MIDI CONTROLLER"` when unplugged.
   - Current buffer latency indicator (e.g., `5.3ms @ 48kHz`).

3. **Master Stereo LED Level Meter (`Meter.tsx`)**:
   - Segmented stereo LED ladder (Green -24dB..-6dB, Yellow -6dB..-1dB, Red 0dB Peak).
   - Driven smoothly by `audio-meter` IPC events.

---

## 3. Acceptance Criteria

- [ ] Window matches the approved Akai Classic Studio visual style.
- [ ] Stereo meter responds dynamically to sound output without dropping frames.
- [ ] Responsive design gracefully scales across standard desktop resolutions.
