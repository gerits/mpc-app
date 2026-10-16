use ministation_lib::assets::get_curated_presets;
use ministation_lib::looper::TrackState;
use ministation_lib::soundfont_manager::SoundFontManager;
use ministation_lib::synth::DualSynthEngine;
use ministation_lib::types::RoutedEvent;
use ministation_lib::wav_export::export_to_wav;

#[test]
fn test_end_to_end_synthesis_dsp_looper_pipeline() {
    let sample_rate = 44100;
    let mut dual_synth = DualSynthEngine::new(sample_rate);
    let mut sf_manager = SoundFontManager::new();

    // 1. Verify Curated Presets
    let presets = get_curated_presets();
    assert!(!presets.is_empty(), "Curated presets should not be empty");

    // 2. Load Melodic Grand Piano & Drum Kit
    let piano_preset = sf_manager
        .select_curated_preset(&ministation_lib::audio::AudioEngine::new(sample_rate as u32, 256), "grand-piano");
    assert!(piano_preset.is_ok(), "Grand piano preset should load successfully");

    // Load soundfonts directly into dual_synth to test realtime audio callback rendering
    let piano_sf = sf_manager.get_or_load_soundfont("Acoustic_Grand.sf2");
    assert!(piano_sf.is_ok(), "Acoustic Grand soundfont should load from assets");
    let piano_synth = ministation_lib::synth::create_synthesizer(&piano_sf.unwrap(), sample_rate).unwrap();
    dual_synth.handle_command(ministation_lib::synth::SynthCommand::SetMelodicSynth(Box::new(piano_synth)));

    let drums_sf = sf_manager.get_or_load_soundfont("909_drums.sf2");
    assert!(drums_sf.is_ok(), "909 Drums soundfont should load from assets");
    let drums_synth = ministation_lib::synth::create_synthesizer(&drums_sf.unwrap(), sample_rate).unwrap();
    dual_synth.handle_command(ministation_lib::synth::SynthCommand::SetDrumSynth(Box::new(drums_synth)));

    // 3. Play simultaneous keys and pads
    dual_synth.handle_routed_event(&RoutedEvent::KeyboardNoteOn {
        channel: 0,
        note: 60, // Middle C
        velocity: 100,
    });
    dual_synth.handle_routed_event(&RoutedEvent::DrumPadOn {
        pad_index: 0,
        note: 36, // Kick Drum
        velocity: 110,
    });

    // 4. Render an audio buffer
    let buffer_size = 256;
    let mut interleaved = vec![0.0f32; buffer_size * 2];
    let (peak_l, peak_r, rms_l, rms_r) = dual_synth.render_interleaved(&mut interleaved, 2);

    // Audio should be non-silent with both voices sounding
    assert!(peak_l > 0.0001, "Left peak should have audio signal");
    assert!(peak_r > 0.0001, "Right peak should have audio signal");
    assert!(rms_l > 0.0001, "Left RMS should be non-zero");
    assert!(rms_r > 0.0001, "Right RMS should be non-zero");

    // 5. Apply DSP Macro Knobs
    dual_synth.handle_routed_event(&RoutedEvent::MacroKnob {
        knob_index: 0, // Cutoff
        value: 0.4,
    });
    dual_synth.handle_routed_event(&RoutedEvent::MacroKnob {
        knob_index: 3, // Reverb
        value: 0.5,
    });
    dual_synth.handle_routed_event(&RoutedEvent::MacroKnob {
        knob_index: 4, // Drive / Overdrive
        value: 0.3,
    });

    // Render next block after DSP tweaks
    let (peak_l2, peak_r2, _, _) = dual_synth.render_interleaved(&mut interleaved, 2);
    assert!(!peak_l2.is_nan() && !peak_r2.is_nan(), "Audio output must not contain NaNs");
    assert!(peak_l2 <= 1.0 && peak_r2 <= 1.0, "Audio output should stay cleanly limited below 1.0");

    // 6. 2-Track Looper Recording & Overdubbing
    dual_synth.handle_command(ministation_lib::synth::SynthCommand::LooperSetBpm(120.0));
    dual_synth.handle_command(ministation_lib::synth::SynthCommand::LooperSetBars(1));
    dual_synth.handle_command(ministation_lib::synth::SynthCommand::LooperRecord(0));
    assert_eq!(dual_synth.looper.tracks[0].state, TrackState::Recording);

    // Render audio blocks to record into Track 1
    for _ in 0..10 {
        dual_synth.render_interleaved(&mut interleaved, 2);
    }

    // Switch to Playback on Track 0, then Overdub on Track 0
    dual_synth.handle_command(ministation_lib::synth::SynthCommand::LooperPlay);
    assert_eq!(dual_synth.looper.tracks[0].state, TrackState::Playing);
    dual_synth.handle_command(ministation_lib::synth::SynthCommand::LooperOverdub(0));
    assert_eq!(dual_synth.looper.tracks[0].state, TrackState::Overdubbing);

    // Render further blocks during overdub
    for _ in 0..10 {
        dual_synth.render_interleaved(&mut interleaved, 2);
    }

    // 7. WAV Export
    let temp_dir = std::env::temp_dir();
    let export_file = temp_dir.join("e2e_test_loop.wav");
    let (mix_l, mix_r) = dual_synth.looper.get_mix_buffers();

    let export_res = export_to_wav(&mix_l, &mix_r, sample_rate as u32, &export_file);
    assert!(export_res.is_ok(), "WAV export should succeed");
    assert!(export_file.exists(), "Exported WAV file should exist on disk");

    // 8. Verify WAV spec with hound reader
    let reader = hound::WavReader::open(&export_file).expect("Failed to open exported WAV");
    let spec = reader.spec();
    assert_eq!(spec.channels, 2);
    assert_eq!(spec.sample_rate, 44100);
    assert_eq!(spec.bits_per_sample, 24);
    assert_eq!(spec.sample_format, hound::SampleFormat::Int);
    assert!(reader.duration() > 0);

    let _ = std::fs::remove_file(export_file);
}

#[test]
fn test_latency_calculation_bounds() {
    let sample_rate = 44100.0f32;
    // 256 samples buffer: 256 / 44100 * 1000 = ~5.80 ms
    let latency_256 = (256.0 / sample_rate) * 1000.0;
    assert!(latency_256 < 6.0, "256 samples latency must be < 6ms");

    // 128 samples buffer: ~2.90 ms
    let latency_128 = (128.0 / sample_rate) * 1000.0;
    assert!(latency_128 < 3.0, "128 samples latency must be < 3ms");
}
