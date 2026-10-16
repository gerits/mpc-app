# MiniStation

> **Lightweight sound module & performance workstation for the Akai MPK mini 3**  
> Built with Tauri v2, Rust (`cpal`, `rustysynth`, `hound`), and React / TypeScript.

MiniStation transforms your **Akai MPK mini 3** (and other MIDI controllers) into an instant, zero-latency standalone sound module. Plug your keyboard in via USB, launch the app, and start jamming immediately — no complex DAW setup, no plugin scanning, and zero external C library dependencies.

---

## ✨ Features

- **Zero-Config Hardware Auto-Detection**: Auto-detects and connects to the Akai MPK mini 3 on launch.
- **Sub-10ms Low Latency**: Native CPAL output engine delivers **~5.8 ms** roundtrip audio latency (256 samples @ 44.1 kHz) down to **~1.5 ms** (64 samples).
- **Dual Pure-Rust SoundFont Synthesizer**: Dedicated independent voices for **Melodic Instruments** (Keyboard keys, Channel 1) and **Drum Kits** (MPC rubber pads, Channel 10) so heavy drum beats never choke sustained piano notes.
- **Hardware Macro DSP Chain**:
  - **K1 (CC 70)**: Cytomic resonant State Variable Filter (20 Hz – 20 kHz smooth sweep).
  - **K2 (CC 71)**: Resonant Q factor (0.7 – 8.0).
  - **K3 (CC 72)**: Stereo chorus with dual LFOs.
  - **K4 (CC 73)**: Freeverb stereo algorithmic reverb.
  - **K5 (CC 74)**: Soft saturator overdrive and anti-clip ceiling limiter.
  - **K6 (CC 75)**: Keys stem gain fader.
  - **K7 (CC 76)**: Drums stem gain fader.
  - **K8 (CC 77)**: Master output volume.
- **2-Track Quick Looper & 24-bit WAV Exporter**:
  - Sample-accurate bar and BPM quantization (1, 2, 4, 8 bars).
  - Micro-crossfade seam smoothing to eliminate boundary wrap-around pops.
  - Real-time overdubbing and per-track mute.
  - Broadcast-standard 24-bit stereo WAV export to `~/Music/MiniStation/`.
- **Akai Classic Studio Chassis**:
  - Authentic matte graphite chassis with chamfered hardware borders.
  - Vintage emerald phosphor CRT / OLED status display.
  - Segmented stereo LED peak & RMS VU meters.
  - 2x4 backlit MPC rubber pads with Bank A/B switching and Full Level mode.
  - 25-key velocity-sensitive visual piano roll preview.

---

## 🎛️ Architecture

```mermaid
flowchart LR
    HW["Akai MPK mini 3 (USB MIDI)"] --> MIDIR["MidiEngine (midir)"]
    MIDIR --> ROUTER["Performance Router"]
    
    subgraph Audio Backend (CPAL Realtime Thread)
        ROUTER -->|Channel 1| MELODIC["Melodic Synth (Keys)"]
        ROUTER -->|Channel 10| DRUM["Drum Synth (Pads)"]
        ROUTER -->|CC 70-77| DSP["DSP Chain (SVF, Chorus, Reverb, Overdrive)"]
        
        MELODIC --> STEMS["Stem Mixer"]
        DRUM --> STEMS
        STEMS --> DSP
        DSP --> LOOPER["2-Track Looper Engine"]
        LOOPER --> CPAL["CPAL Audio Output (Stereo DAC)"]
    end
    
    subgraph UI Frontend (Tauri v2 + React)
        LOOPER -->|WAV Export| FILE["~/Music/MiniStation/*.wav"]
        DSP -.->|AudioMeter @ 30 FPS| METER["Stereo LED VU Meter"]
        ROUTER -.->|MidiEvent < 1ms| PADS["Interactive Pads & Knobs"]
    end
```

---

## 🎹 Akai MPK mini 3 Hardware Mapping

| Control | Parameter / Note | Channel | Description |
|:---|:---|:---:|:---|
| **25 Keys** | Notes 48–72 (C2–C4 default) | Channel 1 | Melodic instrument voice with dynamic velocity |
| **Pitch Bend** | Joystick X-Axis | Channel 1 | Continuous pitch modulation (centered spring) |
| **Modulation** | Joystick Y-Axis | Channel 1 | CC 1 modulation wheel |
| **Pad 1 (Bank A)** | Note #36 (C1) | Channel 10 | Acoustic / 808 / 909 Kick Drum |
| **Pad 2 (Bank A)** | Note #38 (D1) | Channel 10 | Snare Drum |
| **Pad 3 (Bank A)** | Note #42 (F#1) | Channel 10 | Closed Hi-Hat |
| **Pad 4 (Bank A)** | Note #46 (A#1) | Channel 10 | Open Hi-Hat |
| **Pad 5 (Bank A)** | Note #45 (A1) | Channel 10 | Low Tom |
| **Pad 6 (Bank A)** | Note #48 (C2) | Channel 10 | Mid Tom |
| **Pad 7 (Bank A)** | Note #49 (C#2) | Channel 10 | Crash Cymbal |
| **Pad 8 (Bank A)** | Note #51 (D#2) | Channel 10 | Ride Cymbal |
| **Knob K1** | CC 70 | Any | Cytomic SVF Cutoff Frequency |
| **Knob K2** | CC 71 | Any | Resonance Q Factor |
| **Knob K3** | CC 72 | Any | Stereo Chorus Depth |
| **Knob K4** | CC 73 | Any | Freeverb Wet/Dry Mix |
| **Knob K5** | CC 74 | Any | Soft Overdrive Saturation |
| **Knob K6** | CC 75 | Any | Melodic Keys Level |
| **Knob K7** | CC 76 | Any | Drum Kit Level |
| **Knob K8** | CC 77 | Any | Master Output Volume |

---

## 🚀 Quick Start

### Prerequisites
- **macOS**, **Windows 10/11**, or **Linux**
- [Bun](https://bun.sh/) (or Node.js v18+)
- [Rust](https://rustup.rs/) (v1.75+)

### Installation & Run

```bash
# Clone the repository
git clone https://github.com/ruben-gerits/mpc-app.git
cd mpc-app

# Install frontend dependencies
bun install

# Run in development mode (hot-reloading React + Rust)
bun run tauri dev
```

### Build Production Release

```bash
# Compile optimized native binary and desktop installer
bun run tauri build
```
The compiled application bundle will be placed in `src-tauri/target/release/bundle/`.

---

## 🧪 Testing

```bash
# Run all unit tests and e2e integration smoke tests
cargo test --manifest-path src-tauri/Cargo.toml

# Build and verify frontend TypeScript types
bun run build
```

---

## 📦 Bundled SoundFonts & Attribution

MiniStation bundles curated open-source `.sf2` soundfonts to provide immediate out-of-the-box performance without manual asset downloads:

1. **Yamaha C3 Acoustic Grand Piano (`Acoustic_Grand.sf2`)**
   - Source: Open-source multi-sample concert grand piano.
   - License: Public Domain / CC0.
2. **Roland TR-909 & TR-808 Rhythm SoundFont (`909_drums.sf2`)**
   - Source: Standard General MIDI percussion bank mapping.
   - License: CC-BY / Open Source.
3. **General GM SoundFont (`General_GM.sf2`)**
   - Source: Lightweight General MIDI SoundFont covering 128 GM melodic instruments.
   - License: Public Domain / Permissive Open Source.

Users can also import any custom `.sf2` SoundFont by dropping it into `~/Music/MiniStation/SoundFonts/` or using the in-app **SoundFont Manager**.

---

## 📄 License

MIT License. See [LICENSE](LICENSE) for details.
