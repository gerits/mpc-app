use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};

use crate::assets::{CuratedPreset, SoundFontMetadata};
use crate::audio::AudioEngine;
use crate::midi::MidiEngine;
use crate::soundfont_manager::SoundFontManager;

/// Full catalog of SoundFonts and curated performance presets.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoundFontCatalog {
    pub soundfonts: Vec<SoundFontMetadata>,
    pub curated_presets: Vec<CuratedPreset>,
    pub active_melodic_preset_id: Option<String>,
    pub active_drum_preset_id: Option<String>,
}

/// Instantaneous volume settings across channels.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelVolumes {
    pub master: f32,
    pub keys: f32,
    pub drums: f32,
}

/// Overall system runtime status (CPAL audio + MIDI connections).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemStatus {
    pub audio_running: bool,
    pub sample_rate: u32,
    pub buffer_size: u32,
    pub audio_device: Option<String>,
    pub midi_device: Option<String>,
    pub active_melodic_preset: Option<String>,
    pub active_drum_preset: Option<String>,
}

/// Shared application state managed by Tauri.
pub struct AppState {
    pub audio_engine: Arc<Mutex<AudioEngine>>,
    pub midi_engine: Arc<Mutex<MidiEngine>>,
    pub soundfont_manager: Arc<Mutex<SoundFontManager>>,
    pub macro_knobs: Arc<Mutex<[f32; 8]>>,
    pub volumes: Arc<Mutex<ChannelVolumes>>,
}

impl AppState {
    pub fn new(audio_engine: AudioEngine, midi_engine: MidiEngine) -> Self {
        let soundfont_manager = SoundFontManager::new();

        Self {
            audio_engine: Arc::new(Mutex::new(audio_engine)),
            midi_engine: Arc::new(Mutex::new(midi_engine)),
            soundfont_manager: Arc::new(Mutex::new(soundfont_manager)),
            macro_knobs: Arc::new(Mutex::new([0.5, 0.0, 0.5, 0.5, 0.0, 0.0, 0.5, 0.9])),
            volumes: Arc::new(Mutex::new(ChannelVolumes {
                master: 0.9,
                keys: 1.0,
                drums: 1.0,
            })),
        }
    }
}
