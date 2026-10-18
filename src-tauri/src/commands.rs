use std::path::PathBuf;
use tauri::State;

use crate::assets::{CuratedPreset, SoundFontMetadata};
use crate::audio::AudioEngine;
use crate::looper::LooperSnapshot;
use crate::midi::MidiEngine;
use crate::state::{AppState, ChannelVolumes, SoundFontCatalog, SystemStatus};
use crate::types::{MidiDeviceInfo, RoutedEvent};

#[tauri::command]
pub fn get_system_status(state: State<'_, AppState>) -> Result<SystemStatus, String> {
    let audio = state.audio_engine.lock().map_err(|e| e.to_string())?;
    let midi = state.midi_engine.lock().map_err(|e| e.to_string())?;
    let sf = state.soundfont_manager.lock().map_err(|e| e.to_string())?;

    Ok(SystemStatus {
        audio_running: audio.is_running(),
        sample_rate: audio.sample_rate(),
        buffer_size: audio.buffer_size(),
        audio_device: audio.device_name(),
        midi_device: midi.current_device(),
        active_melodic_preset: sf.active_melodic_preset_id().map(String::from),
        active_drum_preset: sf.active_drum_preset_id().map(String::from),
    })
}

#[tauri::command]
pub fn get_audio_devices() -> Result<Vec<String>, String> {
    AudioEngine::list_output_devices()
}

#[tauri::command]
pub fn set_buffer_size(state: State<'_, AppState>, buffer_size: u32) -> Result<(), String> {
    let mut audio = state.audio_engine.lock().map_err(|e| e.to_string())?;
    audio.set_buffer_size(buffer_size)
}

#[tauri::command]
pub fn open_soundfonts_folder() -> Result<String, String> {
    let dir = crate::assets::ensure_user_soundfonts_dir().map_err(|e| e.to_string())?;
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(&dir).spawn();
    }
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("explorer").arg(&dir).spawn();
    }
    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("xdg-open").arg(&dir).spawn();
    }
    Ok(dir.to_string_lossy().to_string())
}

#[tauri::command]
pub fn get_midi_devices() -> Result<Vec<MidiDeviceInfo>, String> {
    MidiEngine::list_devices()
}

#[tauri::command]
pub fn connect_midi_device(
    state: State<'_, AppState>,
    device_name: String,
) -> Result<String, String> {
    let mut midi = state.midi_engine.lock().map_err(|e| e.to_string())?;
    midi.connect_by_name(&device_name)
}

#[tauri::command]
pub fn disconnect_midi_device(state: State<'_, AppState>) -> Result<(), String> {
    let mut midi = state.midi_engine.lock().map_err(|e| e.to_string())?;
    midi.disconnect();
    Ok(())
}

#[tauri::command]
pub fn get_soundfonts(state: State<'_, AppState>) -> Result<SoundFontCatalog, String> {
    let mut sf = state.soundfont_manager.lock().map_err(|e| e.to_string())?;
    let soundfonts = sf.rescan().to_vec();
    let curated_presets = sf.curated_presets().to_vec();
    let active_melodic_preset_id = sf.active_melodic_preset_id().map(String::from);
    let active_drum_preset_id = sf.active_drum_preset_id().map(String::from);

    Ok(SoundFontCatalog {
        soundfonts,
        curated_presets,
        active_melodic_preset_id,
        active_drum_preset_id,
    })
}

#[tauri::command]
pub fn select_curated_preset(
    state: State<'_, AppState>,
    preset_id: String,
) -> Result<CuratedPreset, String> {
    let audio = state.audio_engine.lock().map_err(|e| e.to_string())?;
    let mut sf = state.soundfont_manager.lock().map_err(|e| e.to_string())?;
    sf.select_curated_preset(&audio, &preset_id)
}

#[tauri::command]
pub fn select_soundfont_program(
    state: State<'_, AppState>,
    filename: String,
    is_drum: bool,
    bank: u16,
    patch: u8,
) -> Result<(), String> {
    let audio = state.audio_engine.lock().map_err(|e| e.to_string())?;
    let mut sf = state.soundfont_manager.lock().map_err(|e| e.to_string())?;
    sf.select_soundfont_program(&audio, &filename, is_drum, bank, patch)
}

#[tauri::command]
pub fn import_user_soundfont(
    state: State<'_, AppState>,
    source_path: String,
) -> Result<SoundFontMetadata, String> {
    let mut sf = state.soundfont_manager.lock().map_err(|e| e.to_string())?;
    sf.import_user_soundfont(source_path)
}

#[tauri::command]
pub fn set_macro_parameter(
    state: State<'_, AppState>,
    index: u8,
    value: f32,
) -> Result<(), String> {
    let clamped = value.clamp(0.0, 1.0);
    let audio = state.audio_engine.lock().map_err(|e| e.to_string())?;
    audio.set_macro_knob(index, clamped);

    if let Ok(mut knobs) = state.macro_knobs.lock() {
        if (index as usize) < knobs.len() {
            knobs[index as usize] = clamped;
        }
    }
    Ok(())
}

#[tauri::command]
pub fn get_macro_parameters(state: State<'_, AppState>) -> Result<[f32; 8], String> {
    let knobs = state.macro_knobs.lock().map_err(|e| e.to_string())?;
    Ok(*knobs)
}

#[tauri::command]
pub fn set_volume(
    state: State<'_, AppState>,
    track: String,
    volume: f32,
) -> Result<(), String> {
    let clamped = volume.clamp(0.0, 2.0);
    let audio = state.audio_engine.lock().map_err(|e| e.to_string())?;
    let mut vols = state.volumes.lock().map_err(|e| e.to_string())?;

    match track.to_lowercase().as_str() {
        "master" => {
            audio.set_master_volume(clamped);
            vols.master = clamped;
        }
        "keys" | "melodic" => {
            audio.set_melodic_volume(clamped);
            vols.keys = clamped;
        }
        "drums" | "drum" => {
            audio.set_drum_volume(clamped);
            vols.drums = clamped;
        }
        _ => return Err(format!("Unknown volume track '{track}'")),
    }
    Ok(())
}

#[tauri::command]
pub fn get_volumes(state: State<'_, AppState>) -> Result<ChannelVolumes, String> {
    let vols = state.volumes.lock().map_err(|e| e.to_string())?;
    Ok(vols.clone())
}

#[tauri::command]
pub fn looper_record(state: State<'_, AppState>, track: usize) -> Result<(), String> {
    let audio = state.audio_engine.lock().map_err(|e| e.to_string())?;
    audio.looper_record(track);
    Ok(())
}

#[tauri::command]
pub fn looper_overdub(state: State<'_, AppState>, track: usize) -> Result<(), String> {
    let audio = state.audio_engine.lock().map_err(|e| e.to_string())?;
    audio.looper_overdub(track);
    Ok(())
}

#[tauri::command]
pub fn looper_play(state: State<'_, AppState>) -> Result<(), String> {
    let audio = state.audio_engine.lock().map_err(|e| e.to_string())?;
    audio.looper_play();
    Ok(())
}

#[tauri::command]
pub fn looper_stop(state: State<'_, AppState>) -> Result<(), String> {
    let audio = state.audio_engine.lock().map_err(|e| e.to_string())?;
    audio.looper_stop();
    Ok(())
}

#[tauri::command]
pub fn looper_clear(state: State<'_, AppState>, track: usize) -> Result<(), String> {
    let audio = state.audio_engine.lock().map_err(|e| e.to_string())?;
    audio.looper_clear(track);
    Ok(())
}

#[tauri::command]
pub fn looper_clear_all(state: State<'_, AppState>) -> Result<(), String> {
    let audio = state.audio_engine.lock().map_err(|e| e.to_string())?;
    audio.looper_clear_all();
    Ok(())
}

#[tauri::command]
pub fn looper_set_bpm(state: State<'_, AppState>, bpm: f32) -> Result<(), String> {
    let audio = state.audio_engine.lock().map_err(|e| e.to_string())?;
    audio.looper_set_bpm(bpm);
    Ok(())
}

#[tauri::command]
pub fn looper_set_bars(state: State<'_, AppState>, bars: u32) -> Result<(), String> {
    let audio = state.audio_engine.lock().map_err(|e| e.to_string())?;
    audio.looper_set_bars(bars);
    Ok(())
}

#[tauri::command]
pub fn looper_toggle_mute(state: State<'_, AppState>, track: usize) -> Result<(), String> {
    let audio = state.audio_engine.lock().map_err(|e| e.to_string())?;
    audio.looper_toggle_mute(track);
    Ok(())
}

#[tauri::command]
pub fn looper_set_metronome(state: State<'_, AppState>, enabled: bool) -> Result<(), String> {
    let audio = state.audio_engine.lock().map_err(|e| e.to_string())?;
    audio.looper_set_metronome(enabled);
    Ok(())
}

#[tauri::command]
pub fn looper_set_metronome_volume(state: State<'_, AppState>, volume: f32) -> Result<(), String> {
    let audio = state.audio_engine.lock().map_err(|e| e.to_string())?;
    audio.looper_set_metronome_volume(volume);
    Ok(())
}

#[tauri::command]
pub fn looper_set_metronome_mode(state: State<'_, AppState>, mode: String) -> Result<(), String> {
    let audio = state.audio_engine.lock().map_err(|e| e.to_string())?;
    let m = match mode.to_lowercase().as_str() {
        "always" => crate::metronome::MetronomeMode::Always,
        _ => crate::metronome::MetronomeMode::WhileRunning,
    };
    audio.looper_set_metronome_mode(m);
    Ok(())
}

#[tauri::command]
pub fn looper_export(
    state: State<'_, AppState>,
    custom_path: Option<String>,
) -> Result<String, String> {
    let audio = state.audio_engine.lock().map_err(|e| e.to_string())?;
    let path = audio.export_looper_wav(custom_path.map(PathBuf::from))?;
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn get_looper_snapshot(state: State<'_, AppState>) -> Result<LooperSnapshot, String> {
    let audio = state.audio_engine.lock().map_err(|e| e.to_string())?;
    Ok(audio.looper_snapshot())
}

#[tauri::command]
pub fn panic_audio(state: State<'_, AppState>) -> Result<(), String> {
    let audio = state.audio_engine.lock().map_err(|e| e.to_string())?;
    audio.panic();
    Ok(())
}

// -------------------------------------------------------------------------------------------------
// Realtime UI performance note triggers (for virtual keys and MPC rubber pads clicked in UI)
// -------------------------------------------------------------------------------------------------

#[tauri::command]
pub fn send_note_on(
    state: State<'_, AppState>,
    channel: u8,
    note: u8,
    velocity: u8,
) -> Result<(), String> {
    let audio = state.audio_engine.lock().map_err(|e| e.to_string())?;
    let _ = audio.event_sender().try_send(RoutedEvent::KeyboardNoteOn {
        channel,
        note,
        velocity,
    });
    Ok(())
}

#[tauri::command]
pub fn send_note_off(
    state: State<'_, AppState>,
    channel: u8,
    note: u8,
) -> Result<(), String> {
    let audio = state.audio_engine.lock().map_err(|e| e.to_string())?;
    let _ = audio.event_sender().try_send(RoutedEvent::KeyboardNoteOff {
        channel,
        note,
    });
    Ok(())
}

#[tauri::command]
pub fn send_pad_on(
    state: State<'_, AppState>,
    pad_index: u8,
    note: u8,
    velocity: u8,
) -> Result<(), String> {
    let audio = state.audio_engine.lock().map_err(|e| e.to_string())?;
    let _ = audio.event_sender().try_send(RoutedEvent::DrumPadOn {
        pad_index,
        note,
        velocity,
    });
    Ok(())
}

#[tauri::command]
pub fn send_pad_off(
    state: State<'_, AppState>,
    pad_index: u8,
    note: u8,
) -> Result<(), String> {
    let audio = state.audio_engine.lock().map_err(|e| e.to_string())?;
    let _ = audio.event_sender().try_send(RoutedEvent::DrumPadOff {
        pad_index,
        note,
    });
    Ok(())
}

#[tauri::command]
pub fn send_pitch_bend(state: State<'_, AppState>, value: f32) -> Result<(), String> {
    let audio = state.audio_engine.lock().map_err(|e| e.to_string())?;
    let _ = audio
        .event_sender()
        .try_send(RoutedEvent::PitchBend { value });
    Ok(())
}

#[tauri::command]
pub fn send_modulation(state: State<'_, AppState>, value: f32) -> Result<(), String> {
    let audio = state.audio_engine.lock().map_err(|e| e.to_string())?;
    let _ = audio
        .event_sender()
        .try_send(RoutedEvent::Modulation { value });
    Ok(())
}
