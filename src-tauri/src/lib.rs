use std::sync::Arc;
use tauri::Emitter;

pub mod assets;
pub mod audio;
pub mod commands;
pub mod dsp;
pub mod effects;
pub mod looper;
pub mod midi;
pub mod router;
pub mod soundfont_manager;
pub mod state;
pub mod synth;
pub mod types;
pub mod wav_export;

use audio::AudioEngine;
use midi::MidiEngine;
use state::AppState;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AudioMeterPayload {
    pub peak_left: f32,
    pub peak_right: f32,
    pub rms_left: f32,
    pub rms_right: f32,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let audio_engine = AudioEngine::new(44100, 256);
    let event_sender = audio_engine.event_sender();
    let midi_engine = MidiEngine::new(event_sender);

    let app_state = AppState::new(audio_engine, midi_engine);

    let audio_ref = Arc::clone(&app_state.audio_engine);
    let midi_ref = Arc::clone(&app_state.midi_engine);
    let sf_ref = Arc::clone(&app_state.soundfont_manager);
    let knobs_ref = Arc::clone(&app_state.macro_knobs);

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            commands::get_system_status,
            commands::get_audio_devices,
            commands::set_buffer_size,
            commands::open_soundfonts_folder,
            commands::get_midi_devices,
            commands::connect_midi_device,
            commands::disconnect_midi_device,
            commands::get_soundfonts,
            commands::select_curated_preset,
            commands::select_soundfont_program,
            commands::import_user_soundfont,
            commands::set_macro_parameter,
            commands::get_macro_parameters,
            commands::set_volume,
            commands::get_volumes,
            commands::looper_record,
            commands::looper_overdub,
            commands::looper_play,
            commands::looper_stop,
            commands::looper_clear,
            commands::looper_clear_all,
            commands::looper_set_bpm,
            commands::looper_set_bars,
            commands::looper_toggle_mute,
            commands::looper_export,
            commands::get_looper_snapshot,
            commands::panic_audio,
            commands::send_note_on,
            commands::send_note_off,
            commands::send_pad_on,
            commands::send_pad_off,
            commands::send_pitch_bend,
            commands::send_modulation,
        ])
        .setup(move |app| {
            let handle = app.handle().clone();

            // 1. Start audio output engine
            if let Ok(mut audio) = audio_ref.lock() {
                if let Err(e) = audio.start() {
                    eprintln!("Warning: Failed to start default audio output: {e}");
                }
                // 2. Load curated defaults (Grand Piano + TR-808 Kit)
                if let Ok(mut sf) = sf_ref.lock() {
                    let _ = sf.load_defaults(&audio);
                }
            }

            // 3. Register hardware MIDI event listener callback to dispatch to UI
            let handle_for_midi = handle.clone();
            let knobs_for_midi = Arc::clone(&knobs_ref);
            if let Ok(mut midi) = midi_ref.lock() {
                midi.set_event_callback(Some(Arc::new(move |event| {
                    if let types::RoutedEvent::MacroKnob { knob_index, value } = event {
                        if let Ok(mut knobs) = knobs_for_midi.lock() {
                            if (*knob_index as usize) < knobs.len() {
                                knobs[*knob_index as usize] = *value;
                            }
                        }
                    }
                    let _ = handle_for_midi.emit("midi-event", event);
                })));

                // 4. Auto-connect Akai MPK mini or first MIDI controller
                match midi.auto_connect() {
                    Ok(Some(dev)) => println!("Auto-connected to MIDI device: {dev}"),
                    Ok(None) => println!("No MIDI input device currently detected."),
                    Err(e) => eprintln!("MIDI auto-connect error: {e}"),
                }
            }

            // 5. Spawn background periodic emitter for audio level meters & looper progress (~30 FPS)
            let audio_for_meter = Arc::clone(&audio_ref);
            let handle_for_meter = handle.clone();
            std::thread::spawn(move || {
                loop {
                    std::thread::sleep(std::time::Duration::from_millis(33));
                    if let Ok(audio) = audio_for_meter.lock() {
                        let (pl, pr, rl, rr) = audio.meter().get_levels();
                        let payload = AudioMeterPayload {
                            peak_left: pl,
                            peak_right: pr,
                            rms_left: rl,
                            rms_right: rr,
                        };
                        let _ = handle_for_meter.emit("audio-meter", payload);

                        let snapshot = audio.looper_snapshot();
                        let _ = handle_for_meter.emit("looper-snapshot", snapshot);
                    }
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
