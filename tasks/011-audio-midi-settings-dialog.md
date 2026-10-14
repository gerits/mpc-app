# TASK-011: Audio/MIDI Configuration & Custom SoundFont Dialog

- **Ticket ID**: `TASK-011`
- **Status**: `TODO`
- **Dependencies**: `TASK-004`, `TASK-007`, `TASK-008`
- **Target Files**:
  - `src/components/SettingsModal.tsx`
  - `src/components/SoundFontManager.tsx`

---

## 1. Objective

Provide a settings and configuration modal allowing users to:
1. Select audio output devices and adjust buffer sizes (64, 128, 256, 512 samples) to minimize latency.
2. Select active MIDI controllers or manage multiple devices.
3. Manage SoundFonts: inspect installed soundfonts, open the local SoundFont folder in Finder/Explorer, and drag-and-drop or import custom `.sf2` files.
4. Customize pad note mappings or MIDI channels if using non-default MPK presets.

---

## 2. Implementation Steps

1. **Audio & MIDI Settings Tab**:
   - Audio Output Device dropdown (CoreAudio / WASAPI / ALSA).
   - Sample Rate selector (44.1 kHz, 48 kHz).
   - Buffer Size dropdown with estimated latency calculation (`buffer / sample_rate * 1000 ms`).
   - MIDI Input Port selector with Refresh button.

2. **SoundFont Manager Tab**:
   - List all discovered `.sf2` files with file size, preset count, and path.
   - Button: `"Open SoundFonts Folder"` (uses Tauri shell plugin to reveal folder in Finder).
   - Drag-and-drop zone to import `.sf2` files directly.

3. **MIDI Mapping Tab**:
   - Visual pad assignment table (Pad 1..8 -> MIDI note #36..#51).
   - Toggle: "Auto-detect MPK Mini 3 layout" vs "Standard GM Drum Channel 10".

---

## 3. Acceptance Criteria

- [ ] Changing buffer size updates audio engine latency without crashing.
- [ ] Clicking "Open SoundFonts Folder" opens the correct directory on macOS Finder.
- [ ] Dropping a valid `.sf2` file adds it to the instrument list immediately.
