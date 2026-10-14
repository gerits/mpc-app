# TASK-009: Frontend 2x4 Rubber Pads & Instrument Selector

- **Ticket ID**: `TASK-009`
- **Status**: `TODO`
- **Dependencies**: `TASK-007`, `TASK-008`
- **Target Files**:
  - `src/components/PadMatrix.tsx`
  - `src/components/RubberPad.tsx`
  - `src/components/InstrumentSelector.tsx`
  - `src/components/KeyboardPreview.tsx`

---

## 1. Objective

Build the interactive performance components:
1. The **2x4 MPC Rubber Pad Matrix** representing the 8 pads on the MPK mini 3, which flash brightly with velocity-sensitive glow whenever struck on hardware or clicked on screen.
2. The **Melodic Instrument Browser & Preset Screen** with sound category filters.
3. The **25 Mini-Keys Preview** that highlights piano keys in real time as they are played.

---

## 2. Implementation Steps

1. **2x4 MPC Rubber Pad Matrix (`PadMatrix.tsx`, `RubberPad.tsx`)**:
   - Render 8 pads laid out in 2 rows of 4 (Pads 5–8 top, Pads 1–4 bottom, matching MPK mini 3 hardware numbering).
   - Display Pad label, Drum sound name (e.g., Kick, Snare, Clap, Closed Hat, Open Hat, Tom, Crash, Ride), and MIDI Note number (#36–#51).
   - Bank A / Bank B toggle button.
   - Listen to `midi-event` from Tauri; trigger CSS pulse and border glow on hit.
   - Support mouse click / touch trigger for testing directly from screen.

2. **Instrument Selector & Category Filter (`InstrumentSelector.tsx`)**:
   - Amber OLED preset screen displaying active soundfont name.
   - Category filter pills: `All`, `Keys`, `Pads`, `Leads`, `Bass`.
   - Dropdown or quick left/right preset navigation buttons.

3. **25-Key Interactive Preview (`KeyboardPreview.tsx`)**:
   - Render 25 keys (2 full octaves C to C).
   - Light up keys when MIDI note-on events arrive on Channel 1.
   - Allow clicking keys with mouse to trigger notes.

---

## 3. Acceptance Criteria

- [ ] Striking Pad 1 on the MPK mini 3 lights up Pad 1 in the UI instantaneously.
- [ ] Changing melodic instrument loads the new SoundFont preset immediately.
- [ ] Clicking on-screen pads or keys plays the corresponding sound via Tauri IPC.
