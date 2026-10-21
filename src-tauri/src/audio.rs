use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{BufferSize, OutputCallbackInfo, SampleFormat, Stream, StreamConfig};
use crossbeam_channel::{Receiver, Sender};
use rustysynth::SoundFont;

use crate::looper::LooperSnapshot;
use crate::synth::{
    create_synthesizer, load_soundfont_from_bytes, load_soundfont_from_path, AudioMeter,
    DualSynthEngine, SynthCommand,
};
use crate::types::RoutedEvent;

/// Configuration parameters for initializing or switching audio streams.
#[derive(Debug, Clone)]
pub struct AudioStreamConfig {
    pub sample_rate: u32,
    pub buffer_size: u32,
    pub channels: u16,
    pub device_name: Option<String>,
}

impl Default for AudioStreamConfig {
    fn default() -> Self {
        Self {
            sample_rate: 44100,
            buffer_size: 512,
            channels: 2,
            device_name: None,
        }
    }
}

/// Core Audio Output Engine wrapping CPAL and hosting the lock-free DualSynthEngine.
pub struct AudioEngine {
    stream: Option<Stream>,
    command_sender: Sender<SynthCommand>,
    event_sender: Sender<RoutedEvent>,
    event_receiver: Receiver<RoutedEvent>,
    meter: Arc<AudioMeter>,
    looper_snapshot: Arc<Mutex<LooperSnapshot>>,
    config: AudioStreamConfig,
    current_sample_rate: Arc<AtomicU32>,
    current_buffer_size: Arc<AtomicU32>,
    current_device_name: Arc<Mutex<Option<String>>>,
    is_running: Arc<AtomicBool>,
}

impl AudioEngine {
    /// Create a new `AudioEngine` with internal communication channels and default configuration.
    pub fn new(sample_rate: u32, buffer_size: u32) -> Self {
        let (command_sender, _command_receiver) = crossbeam_channel::unbounded::<SynthCommand>();
        let (event_sender, event_receiver) = crossbeam_channel::unbounded::<RoutedEvent>();

        Self {
            stream: None,
            command_sender,
            event_sender,
            event_receiver,
            meter: Arc::new(AudioMeter::new()),
            looper_snapshot: Arc::new(Mutex::new(LooperSnapshot::default())),
            config: AudioStreamConfig {
                sample_rate,
                buffer_size,
                channels: 2,
                device_name: None,
            },
            current_sample_rate: Arc::new(AtomicU32::new(sample_rate)),
            current_buffer_size: Arc::new(AtomicU32::new(buffer_size)),
            current_device_name: Arc::new(Mutex::new(None)),
            is_running: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Enumerate all available audio output devices on the current host.
    pub fn list_output_devices() -> Result<Vec<String>, String> {
        let host = cpal::default_host();
        let devices = host
            .output_devices()
            .map_err(|e| format!("Failed to enumerate output devices: {e}"))?;

        let mut names = Vec::new();
        for dev in devices {
            if let Ok(desc) = dev.description() {
                names.push(desc.name().to_string());
            }
        }
        Ok(names)
    }

    /// Get sender channel for performance routed events (keys, pads, pitch bend, knobs).
    pub fn event_sender(&self) -> Sender<RoutedEvent> {
        self.event_sender.clone()
    }

    /// Get reference to lock-free audio level meter.
    pub fn meter(&self) -> Arc<AudioMeter> {
        Arc::clone(&self.meter)
    }

    /// Returns whether the audio stream is currently active.
    pub fn is_running(&self) -> bool {
        self.is_running.load(Ordering::Relaxed)
    }

    /// Returns the currently active audio device name.
    pub fn device_name(&self) -> Option<String> {
        self.current_device_name.lock().ok()?.clone()
    }

    /// Returns current audio sample rate in Hz.
    pub fn sample_rate(&self) -> u32 {
        self.current_sample_rate.load(Ordering::Relaxed)
    }

    /// Returns current audio buffer size in samples.
    pub fn buffer_size(&self) -> u32 {
        self.current_buffer_size.load(Ordering::Relaxed)
    }

    /// Starts the low-latency CPAL output stream on the default audio output device.
    pub fn start(&mut self) -> Result<(), String> {
        self.stop();

        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or_else(|| "No default audio output device found on system".to_string())?;

        let dev_name = device
            .description()
            .map(|d| d.name().to_string())
            .unwrap_or_else(|_| "Default Audio Device".to_string());

        let supported_config = device
            .default_output_config()
            .map_err(|e| format!("Failed to get default output config: {e}"))?;

        let sample_format = supported_config.sample_format();
        let channels = (supported_config.channels() as usize).min(2);
        let sample_rate = self.config.sample_rate;
        let buffer_size = self.config.buffer_size;

        let (command_tx, command_rx) = crossbeam_channel::unbounded::<SynthCommand>();
        self.command_sender = command_tx;

        let event_rx = self.event_receiver.clone();
        let meter = Arc::clone(&self.meter);
        let looper_snap = Arc::clone(&self.looper_snapshot);
        let err_fn = |err: cpal::Error| match err.kind() {
            cpal::ErrorKind::Xrun => {
                // Non-fatal transient buffer underrun/overrun notification from hardware driver.
                // Audio continues streaming without interruption.
            }
            _ => {
                eprintln!("Audio output stream error: {err}");
            }
        };

        let mut dual_synth = DualSynthEngine::new(sample_rate as i32);

        // Attempt low-latency fixed buffer size first; fallback to driver default if unsupported
        let mut stream_config: StreamConfig = supported_config.into();
        stream_config.channels = channels as u16;
        stream_config.sample_rate = sample_rate;
        stream_config.buffer_size = BufferSize::Fixed(buffer_size);

        let looper_snap_cb = Arc::clone(&looper_snap);
        let mut snap_count_main: u32 = 0;
        let build_result = match sample_format {
            SampleFormat::F32 => device.build_output_stream(
                stream_config.clone(),
                move |data: &mut [f32], _: &OutputCallbackInfo| {
                    while let Ok(cmd) = command_rx.try_recv() {
                        dual_synth.handle_command(cmd);
                    }
                    while let Ok(event) = event_rx.try_recv() {
                        dual_synth.handle_routed_event(&event);
                    }
                    let (peak_l, peak_r, rms_l, rms_r) =
                        dual_synth.render_interleaved(data, channels);
                    meter.update(peak_l, peak_r, rms_l, rms_r);
                    snap_count_main += 1;
                    if snap_count_main >= 6 {
                        snap_count_main = 0;
                        if let Ok(mut snap) = looper_snap_cb.try_lock() {
                            *snap = dual_synth.looper.get_snapshot();
                        }
                    }
                },
                err_fn,
                None,
            ),
            _ => {
                let mut fallback_config = stream_config.clone();
                fallback_config.buffer_size = BufferSize::Default;
                let looper_snap_fb1 = Arc::clone(&looper_snap);
                let mut snap_count_fb1: u32 = 0;
                device.build_output_stream(
                    fallback_config,
                    move |data: &mut [f32], _: &OutputCallbackInfo| {
                        while let Ok(cmd) = command_rx.try_recv() {
                            dual_synth.handle_command(cmd);
                        }
                        while let Ok(event) = event_rx.try_recv() {
                            dual_synth.handle_routed_event(&event);
                        }
                        let (peak_l, peak_r, rms_l, rms_r) =
                            dual_synth.render_interleaved(data, channels);
                        meter.update(peak_l, peak_r, rms_l, rms_r);
                        snap_count_fb1 += 1;
                        if snap_count_fb1 >= 6 {
                            snap_count_fb1 = 0;
                            if let Ok(mut snap) = looper_snap_fb1.try_lock() {
                                *snap = dual_synth.looper.get_snapshot();
                            }
                        }
                    },
                    err_fn,
                    None,
                )
            }
        };

        let stream = match build_result {
            Ok(s) => s,
            Err(e) => {
                // Retry with driver default buffer size if fixed buffer was rejected
                let mut fallback_config = stream_config;
                fallback_config.buffer_size = BufferSize::Default;

                let (cmd_tx, cmd_rx) = crossbeam_channel::unbounded::<SynthCommand>();
                self.command_sender = cmd_tx;
                let event_rx_fb = self.event_receiver.clone();
                let meter_fb = Arc::clone(&self.meter);
                let looper_snap_fb2 = Arc::clone(&self.looper_snapshot);
                let mut dual_synth_fb = DualSynthEngine::new(sample_rate as i32);
                let mut snap_count_fb2: u32 = 0;

                device
                    .build_output_stream(
                        fallback_config,
                        move |data: &mut [f32], _: &OutputCallbackInfo| {
                            while let Ok(cmd) = cmd_rx.try_recv() {
                                dual_synth_fb.handle_command(cmd);
                            }
                            while let Ok(event) = event_rx_fb.try_recv() {
                                dual_synth_fb.handle_routed_event(&event);
                            }
                            let (peak_l, peak_r, rms_l, rms_r) =
                                dual_synth_fb.render_interleaved(data, channels);
                            meter_fb.update(peak_l, peak_r, rms_l, rms_r);
                            snap_count_fb2 += 1;
                            if snap_count_fb2 >= 6 {
                                snap_count_fb2 = 0;
                                if let Ok(mut snap) = looper_snap_fb2.try_lock() {
                                    *snap = dual_synth_fb.looper.get_snapshot();
                                }
                            }
                        },
                        err_fn,
                        None,
                    )
                    .map_err(|err| {
                        format!("Failed to build audio output stream ({e}, fallback failed: {err})")
                    })?
            }
        };

        stream
            .play()
            .map_err(|e| format!("Failed to start audio playback stream: {e}"))?;

        if let Ok(mut lock) = self.current_device_name.lock() {
            *lock = Some(dev_name);
        }
        self.current_sample_rate.store(sample_rate, Ordering::Relaxed);
        self.current_buffer_size.store(buffer_size, Ordering::Relaxed);
        self.is_running.store(true, Ordering::Relaxed);
        self.stream = Some(stream);

        Ok(())
    }

    /// Stops audio playback and drops the active stream.
    pub fn stop(&mut self) {
        if let Some(stream) = self.stream.take() {
            let _ = stream.pause();
        }
        self.is_running.store(false, Ordering::Relaxed);
    }

    /// Change buffer size dynamically and restart the stream if active.
    pub fn set_buffer_size(&mut self, buffer_size: u32) -> Result<(), String> {
        let running = self.is_running();
        self.config.buffer_size = buffer_size;
        if running {
            self.start()?;
        } else {
            self.current_buffer_size.store(buffer_size, Ordering::Relaxed);
        }
        Ok(())
    }

    /// Hot-swaps the melodic soundfont lock-free without stopping audio or interrupting drums.
    pub fn load_melodic_soundfont(&self, soundfont: Arc<SoundFont>) -> Result<(), String> {
        let sample_rate = self.sample_rate() as i32;
        let synth = create_synthesizer(&soundfont, sample_rate)
            .map_err(|e| format!("Failed to create melodic synthesizer: {e:?}"))?;
        let _ = self
            .command_sender
            .send(SynthCommand::SetMelodicSynth(Box::new(synth)));
        Ok(())
    }

    /// Hot-swaps the drum kit soundfont lock-free without interrupting melodic playback.
    pub fn load_drum_soundfont(&self, soundfont: Arc<SoundFont>) -> Result<(), String> {
        let sample_rate = self.sample_rate() as i32;
        let synth = create_synthesizer(&soundfont, sample_rate)
            .map_err(|e| format!("Failed to create drum synthesizer: {e:?}"))?;
        let _ = self
            .command_sender
            .send(SynthCommand::SetDrumSynth(Box::new(synth)));
        Ok(())
    }

    /// Load melodic soundfont from memory bytes and hot-swap.
    pub fn load_melodic_soundfont_from_bytes(&self, bytes: &[u8]) -> Result<(), String> {
        let sf = load_soundfont_from_bytes(bytes)
            .map_err(|e| format!("Failed to parse SoundFont: {e:?}"))?;
        self.load_melodic_soundfont(sf)
    }

    /// Load drum soundfont from memory bytes and hot-swap.
    pub fn load_drum_soundfont_from_bytes(&self, bytes: &[u8]) -> Result<(), String> {
        let sf = load_soundfont_from_bytes(bytes)
            .map_err(|e| format!("Failed to parse Drum SoundFont: {e:?}"))?;
        self.load_drum_soundfont(sf)
    }

    /// Load melodic soundfont from file path and hot-swap.
    pub fn load_melodic_soundfont_from_path<P: AsRef<Path>>(&self, path: P) -> Result<(), String> {
        let sf = load_soundfont_from_path(path)
            .map_err(|e| format!("Failed to load SoundFont file: {e}"))?;
        self.load_melodic_soundfont(sf)
    }

    /// Load drum soundfont from file path and hot-swap.
    pub fn load_drum_soundfont_from_path<P: AsRef<Path>>(&self, path: P) -> Result<(), String> {
        let sf = load_soundfont_from_path(path)
            .map_err(|e| format!("Failed to load Drum SoundFont file: {e}"))?;
        self.load_drum_soundfont(sf)
    }

    /// Clear melodic synthesizer (mute melodic instruments).
    pub fn clear_melodic_synth(&self) {
        let _ = self.command_sender.send(SynthCommand::ClearMelodicSynth);
    }

    /// Clear drum kit synthesizer (mute pads).
    pub fn clear_drum_synth(&self) {
        let _ = self.command_sender.send(SynthCommand::ClearDrumSynth);
    }

    /// Set volume for melodic instrument track (0.0 ..= 2.0).
    pub fn set_melodic_volume(&self, volume: f32) {
        let _ = self
            .command_sender
            .send(SynthCommand::SetMelodicVolume(volume));
    }

    /// Set volume for drum kit track (0.0 ..= 2.0).
    pub fn set_drum_volume(&self, volume: f32) {
        let _ = self
            .command_sender
            .send(SynthCommand::SetDrumVolume(volume));
    }

    /// Set master output volume (0.0 ..= 2.0).
    pub fn set_master_volume(&self, volume: f32) {
        let _ = self
            .command_sender
            .send(SynthCommand::SetMasterVolume(volume));
    }

    /// Select melodic program (bank and patch).
    pub fn set_melodic_program(&self, channel: u8, bank: u16, program: u8) {
        let _ = self.command_sender.send(SynthCommand::SetMelodicProgram {
            channel,
            bank,
            program,
        });
    }

    /// Select drum kit program (bank and patch).
    pub fn set_drum_program(&self, bank: u16, program: u8) {
        let _ = self
            .command_sender
            .send(SynthCommand::SetDrumProgram { bank, program });
    }

    /// Set macro rotary knob parameter 0..=7 (CC 70..=77).
    pub fn set_macro_knob(&self, knob_index: u8, value: f32) {
        let _ = self
            .command_sender
            .send(SynthCommand::SetMacroKnob { knob_index, value });
    }

    /// Panic: immediately stop all sounding notes and reset controllers.
    pub fn panic(&self) {
        let _ = self.command_sender.send(SynthCommand::Panic);
    }

    /// Read the latest looper status snapshot.
    pub fn looper_snapshot(&self) -> LooperSnapshot {
        self.looper_snapshot
            .lock()
            .map(|s| s.clone())
            .unwrap_or_default()
    }

    /// Arm or toggle Record on track (0 or 1).
    pub fn looper_record(&self, track_idx: usize) {
        let _ = self.command_sender.send(SynthCommand::LooperRecord(track_idx));
    }

    /// Arm or toggle Record on BOTH tracks simultaneously.
    pub fn looper_record_both(&self) {
        let _ = self.command_sender.send(SynthCommand::LooperRecordBoth);
    }

    /// Arm or toggle Overdub on track (0 or 1).
    pub fn looper_overdub(&self, track_idx: usize) {
        let _ = self.command_sender.send(SynthCommand::LooperOverdub(track_idx));
    }

    /// Start or resume looper transport playback.
    pub fn looper_play(&self) {
        let _ = self.command_sender.send(SynthCommand::LooperPlay);
    }

    /// Stop looper transport playback.
    pub fn looper_stop(&self) {
        let _ = self.command_sender.send(SynthCommand::LooperStop);
    }

    /// Clear audio buffer of a specific track (0 or 1).
    pub fn looper_clear(&self, track_idx: usize) {
        let _ = self.command_sender.send(SynthCommand::LooperClear(track_idx));
    }

    /// Clear all tracks and reset playhead.
    pub fn looper_clear_all(&self) {
        let _ = self.command_sender.send(SynthCommand::LooperClearAll);
    }

    /// Set looper BPM tempo.
    pub fn looper_set_bpm(&self, bpm: f32) {
        let _ = self.command_sender.send(SynthCommand::LooperSetBpm(bpm));
    }

    /// Set loop bar length (1, 2, 4, 8).
    pub fn looper_set_bars(&self, bars: u32) {
        let _ = self.command_sender.send(SynthCommand::LooperSetBars(bars));
    }

    /// Toggle mute on track (0 or 1).
    pub fn looper_toggle_mute(&self, track_idx: usize) {
        let _ = self
            .command_sender
            .send(SynthCommand::LooperToggleMute(track_idx));
    }

    /// Enable or disable metronome click.
    pub fn looper_set_metronome(&self, enabled: bool) {
        let _ = self
            .command_sender
            .send(SynthCommand::LooperSetMetronome(enabled));
    }

    /// Set metronome click volume (0.0 ..= 2.0).
    pub fn looper_set_metronome_volume(&self, volume: f32) {
        let _ = self
            .command_sender
            .send(SynthCommand::LooperSetMetronomeVolume(volume));
    }

    /// Set metronome mode (while_running or always).
    pub fn looper_set_metronome_mode(&self, mode: crate::metronome::MetronomeMode) {
        let _ = self
            .command_sender
            .send(SynthCommand::LooperSetMetronomeMode(mode));
    }

    /// Update note position or pitch in a looper track.
    pub fn looper_update_note(&self, track: usize, note_id: String, new_step: f32, new_note: u8) {
        let _ = self.command_sender.send(SynthCommand::UpdateLoopNote {
            track,
            note_id,
            new_step,
            new_note,
        });
    }

    /// Quantize all notes in a looper track to the specified grid step.
    pub fn looper_quantize(&self, track: usize, grid_step: f32) {
        let _ = self.command_sender.send(SynthCommand::QuantizeLoop {
            track,
            grid_step,
        });
    }

    /// Delete a single note in a looper track.
    pub fn looper_delete_note(&self, track: usize, note_id: String) {
        let _ = self.command_sender.send(SynthCommand::DeleteLoopNote {
            track,
            note_id,
        });
    }

    /// Clear all sequenced notes in a looper track.
    pub fn looper_clear_notes(&self, track: usize) {
        let _ = self
            .command_sender
            .send(SynthCommand::ClearLoopNotes(track));
    }

    /// Export the combined looper audio mix to a 24-bit broadcast WAV file.
    pub fn export_looper_wav(&self, custom_path: Option<PathBuf>) -> Result<PathBuf, String> {
        let (tx, rx) = crossbeam_channel::bounded(1);
        self.command_sender
            .send(SynthCommand::LooperGetMixBuffers(tx))
            .map_err(|e| format!("Audio engine is not running: {e}"))?;

        let (mix_l, mix_r, sample_rate) = rx
            .recv_timeout(std::time::Duration::from_millis(500))
            .map_err(|_| "Timed out waiting for looper mix buffers from audio thread".to_string())?;

        let has_signal = mix_l.iter().chain(mix_r.iter()).any(|&s| s.abs() > 0.0001);
        if !has_signal {
            return Err("No recorded loop audio found to export.".to_string());
        }

        let target_path = match custom_path {
            Some(p) => p,
            None => {
                let dir = crate::wav_export::get_default_recordings_dir();
                let snapshot = self.looper_snapshot();
                let filename = crate::wav_export::generate_timestamped_filename(snapshot.bpm);
                dir.join(filename)
            }
        };

        crate::wav_export::export_to_wav(&mix_l, &mix_r, sample_rate as u32, &target_path)?;
        Ok(target_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_engine_instantiation() {
        let engine = AudioEngine::new(44100, 256);
        assert_eq!(engine.is_running(), false);
        assert_eq!(engine.sample_rate(), 44100);
        assert_eq!(engine.buffer_size(), 256);
        assert_eq!(engine.device_name(), None);

        let (pl, pr, _, _) = engine.meter().get_levels();
        assert_eq!(pl, 0.0);
        assert_eq!(pr, 0.0);
    }

    #[test]
    fn test_list_output_devices() {
        // Enumerate devices on host (passes even in CI if devices empty or populated)
        let devices = AudioEngine::list_output_devices();
        assert!(devices.is_ok());
    }

    #[test]
    fn test_audio_engine_lifecycle() {
        let mut engine = AudioEngine::new(44100, 256);
        // Attempt start on host default output device
        if let Ok(()) = engine.start() {
            assert!(engine.is_running());
            assert!(engine.device_name().is_some());

            // Send test performance events down the lock-free event sender
            let sender = engine.event_sender();
            let _ = sender.send(RoutedEvent::KeyboardNoteOn {
                channel: 0,
                note: 60,
                velocity: 100,
            });
            let _ = sender.send(RoutedEvent::DrumPadOn {
                pad_index: 0,
                note: 36,
                velocity: 110,
            });

            // Let the callback process a few blocks
            std::thread::sleep(std::time::Duration::from_millis(50));

            engine.set_melodic_volume(0.8);
            engine.set_drum_volume(0.9);
            engine.set_master_volume(0.95);
            engine.panic();

            engine.stop();
            assert_eq!(engine.is_running(), false);
        }
    }

    #[test]
    fn test_audio_engine_looper_controls() {
        let engine = AudioEngine::new(44100, 256);
        let snap = engine.looper_snapshot();
        assert_eq!(snap.bpm, 120.0);
        assert_eq!(snap.bar_count, 4);
        assert_eq!(snap.is_running, false);

        engine.looper_set_bpm(128.0);
        engine.looper_set_bars(2);
        engine.looper_record(0);
        engine.looper_overdub(1);
        engine.looper_toggle_mute(0);
        engine.looper_play();
        engine.looper_stop();
        engine.looper_clear(0);
        engine.looper_clear_all();
    }
}
