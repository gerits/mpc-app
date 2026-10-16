# TASK-004: Curated Open-Source SoundFonts & Asset Loader

- **Ticket ID**: `TASK-004`
- **Status**: `DONE`
- **Dependencies**: `TASK-001`, `TASK-003`
- **Target Files**:
  - `src-tauri/assets/soundfonts/`
  - `src-tauri/src/assets.rs`
  - `src-tauri/src/soundfont_manager.rs`

---

## 1. Objective

Provide a set of high-quality, open-source (CC0 / Public Domain / MIT) SoundFont (.sf2) files bundled with the application for instant plug-and-play operation, and implement a flexible SoundFont loader that discovers both bundled presets and user-added `.sf2` files from an application data directory.

---

## 2. Implementation Steps

1. **Curated Open-Source SoundFont Collection**:
   - Source lightweight, expressive SoundFonts:
     - **Grand Piano**: High-clarity acoustic concert grand.
     - **Electric Piano / Rhodes**: Warm vintage tine piano.
     - **Synthesizer Lead / Pad**: Warm analog filter sweeps and lush pads.
     - **Bass**: Round acoustic or electric bass.
     - **TR-808 & 909 Kit**: Classic electronic drums mapped to GM drum notes 36–51.
     - **Acoustic Studio Drum Kit**: Natural acoustic kick, snare, cymbals, and toms.
   - Store default soundfonts in `src-tauri/assets/soundfonts/`.

2. **SoundFont Catalog & Metadata Indexing (`assets.rs`)**:
   - Index soundfonts with metadata:
     - Name, Category (Piano, Electric Piano, Synth, Bass, Drum Kit), Author/License, File path.
   - Scan an external user directory (e.g., `~/Library/Application Support/MiniStation/soundfonts` on macOS) for user-dropped `.sf2` files.

3. **Dynamic SoundFont Switching**:
   - Implement hot-swapping in the synth engine without restarting the audio stream.

---

## 3. Acceptance Criteria

- [x] App launches with bundled default SoundFonts available immediately out-of-the-box.
- [x] Switching between Grand Piano, Rhodes, Synth, and Bass takes <50ms without audio glitches.
- [x] Dropping a third-party `.sf2` file into the user directory makes it selectable in the app.
