use std::fs::File;
use std::io::{Cursor, Read};
use std::path::Path;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use rustysynth::{SoundFont, SoundFontError, Synthesizer, SynthesizerError, SynthesizerSettings};

use crate::dsp::DspChain;
use crate::looper::LooperEngine;
use crate::types::{MidiEvent, RoutedEvent};

/// Realtime commands sent from host/UI threads to the audio synthesis engine.
#[derive(Debug)]
pub enum SynthCommand {
    /// Hot-swap or install the melodic synthesizer
    SetMelodicSynth(Box<Synthesizer>),
    /// Hot-swap or install the drum kit synthesizer
    SetDrumSynth(Box<Synthesizer>),
    /// Remove/mute the melodic synthesizer
    ClearMelodicSynth,
    /// Remove/mute the drum kit synthesizer
    ClearDrumSynth,
    /// Set melodic track volume (0.0 ..= 2.0, default 1.0)
    SetMelodicVolume(f32),
    /// Set drum track volume (0.0 ..= 2.0, default 1.0)
    SetDrumVolume(f32),
    /// Set master output volume (0.0 ..= 2.0, default 0.9)
    SetMasterVolume(f32),
    /// Select program/patch and bank for melodic instrument
    SetMelodicProgram { channel: u8, bank: u16, program: u8 },
    /// Select program/patch and bank for drum kit
    SetDrumProgram { bank: u16, program: u8 },
    /// Set macro rotary knob parameter 0..=7 (CC 70..=77)
    SetMacroKnob { knob_index: u8, value: f32 },
    /// Arm or toggle Record on looper track (0 or 1)
    LooperRecord(usize),
    /// Arm or toggle Record on BOTH looper tracks simultaneously
    LooperRecordBoth,
    /// Arm or toggle Overdub on looper track (0 or 1)
    LooperOverdub(usize),
    /// Start looper playback
    LooperPlay,
    /// Stop looper playback and rewind
    LooperStop,
    /// Clear a specific looper track
    LooperClear(usize),
    /// Clear all looper tracks
    LooperClearAll,
    /// Set looper BPM
    LooperSetBpm(f32),
    /// Set looper bar length (1, 2, 4, 8)
    LooperSetBars(u32),
    /// Toggle mute on a looper track
    LooperToggleMute(usize),
    /// Enable or disable looper metronome
    LooperSetMetronome(bool),
    /// Set looper metronome volume (0.0 ..= 2.0)
    LooperSetMetronomeVolume(f32),
    /// Set looper metronome mode (while_running or always)
    LooperSetMetronomeMode(crate::metronome::MetronomeMode),
    /// Retrieve full mix buffers for WAV export
    LooperGetMixBuffers(crossbeam_channel::Sender<(Vec<f32>, Vec<f32>, f32)>),
    /// Update note timing / pitch when dragged in visualizer
    UpdateLoopNote {
        track: usize,
        note_id: String,
        new_step: f32,
        new_note: u8,
    },
    /// Quantize all notes on track
    QuantizeLoop {
        track: usize,
        grid_step: f32,
    },
    /// Delete a single note on track
    DeleteLoopNote {
        track: usize,
        note_id: String,
    },
    /// Clear all notes on a track
    ClearLoopNotes(usize),
    /// Silence all active voices and reset MIDI controller parameters
    Panic,
}

/// Lock-free peak and RMS audio level meter updated per audio block.
#[derive(Debug, Default)]
pub struct AudioMeter {
    peak_left: AtomicU32,
    peak_right: AtomicU32,
    rms_left: AtomicU32,
    rms_right: AtomicU32,
}

impl AudioMeter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Update peak and RMS levels atomically.
    pub fn update(&self, peak_left: f32, peak_right: f32, rms_left: f32, rms_right: f32) {
        self.peak_left.store(peak_left.to_bits(), Ordering::Relaxed);
        self.peak_right
            .store(peak_right.to_bits(), Ordering::Relaxed);
        self.rms_left.store(rms_left.to_bits(), Ordering::Relaxed);
        self.rms_right.store(rms_right.to_bits(), Ordering::Relaxed);
    }

    /// Read current meter levels `(peak_left, peak_right, rms_left, rms_right)`.
    pub fn get_levels(&self) -> (f32, f32, f32, f32) {
        (
            f32::from_bits(self.peak_left.load(Ordering::Relaxed)),
            f32::from_bits(self.peak_right.load(Ordering::Relaxed)),
            f32::from_bits(self.rms_left.load(Ordering::Relaxed)),
            f32::from_bits(self.rms_right.load(Ordering::Relaxed)),
        )
    }
}

/// Dual-synth engine running inside the realtime audio callback.
/// Separates melodic instruments (keyboard keys) from percussion instruments (MPC rubber pads)
/// so that drum hits never cut off sustained melodic notes or conflict with melodic soundfont banks.
pub struct DualSynthEngine {
    pub sample_rate: i32,
    pub dsp: DspChain,
    pub looper: LooperEngine,
    melodic_synth: Option<Box<Synthesizer>>,
    drum_synth: Option<Box<Synthesizer>>,
    melodic_volume: f32,
    drum_volume: f32,
    master_volume: f32,

    // Pre-allocated scratch buffers to eliminate allocations during the audio callback
    scratch_mel_left: Vec<f32>,
    scratch_mel_right: Vec<f32>,
    scratch_drm_left: Vec<f32>,
    scratch_drm_right: Vec<f32>,
    scratch_out_left: Vec<f32>,
    scratch_out_right: Vec<f32>,
}

impl DualSynthEngine {
    /// Standard MIDI channel for percussion (channel 10 in 1-based, 9 in 0-based).
    pub const PERCUSSION_CHANNEL: i32 = 9;

    pub fn new(sample_rate: i32) -> Self {
        const INITIAL_CAPACITY: usize = 1024;
        Self {
            sample_rate,
            dsp: DspChain::new(sample_rate as f32),
            looper: LooperEngine::new(sample_rate as f32),
            melodic_synth: None,
            drum_synth: None,
            melodic_volume: 1.0,
            drum_volume: 1.0,
            master_volume: 0.9,
            scratch_mel_left: vec![0.0; INITIAL_CAPACITY],
            scratch_mel_right: vec![0.0; INITIAL_CAPACITY],
            scratch_drm_left: vec![0.0; INITIAL_CAPACITY],
            scratch_drm_right: vec![0.0; INITIAL_CAPACITY],
            scratch_out_left: vec![0.0; INITIAL_CAPACITY],
            scratch_out_right: vec![0.0; INITIAL_CAPACITY],
        }
    }

    /// Change sample rate. Existing synthesizers should be recreated if sample rate changes.
    pub fn set_sample_rate(&mut self, sample_rate: i32) {
        self.sample_rate = sample_rate;
        self.dsp.set_sample_rate(sample_rate as f32);
        self.looper.set_sample_rate(sample_rate as f32);
    }

    pub fn has_melodic_synth(&self) -> bool {
        self.melodic_synth.is_some()
    }

    pub fn has_drum_synth(&self) -> bool {
        self.drum_synth.is_some()
    }

    pub fn melodic_volume(&self) -> f32 {
        self.melodic_volume
    }

    pub fn drum_volume(&self) -> f32 {
        self.drum_volume
    }

    pub fn master_volume(&self) -> f32 {
        self.master_volume
    }

    /// Process a control command received from host/UI threads.
    pub fn handle_command(&mut self, command: SynthCommand) {
        match command {
            SynthCommand::SetMelodicSynth(synth) => {
                self.melodic_synth = Some(synth);
            }
            SynthCommand::SetDrumSynth(synth) => {
                self.drum_synth = Some(synth);
            }
            SynthCommand::ClearMelodicSynth => {
                self.melodic_synth = None;
            }
            SynthCommand::ClearDrumSynth => {
                self.drum_synth = None;
            }
            SynthCommand::SetMelodicVolume(vol) => {
                self.melodic_volume = vol.clamp(0.0, 2.0);
            }
            SynthCommand::SetDrumVolume(vol) => {
                self.drum_volume = vol.clamp(0.0, 2.0);
            }
            SynthCommand::SetMasterVolume(vol) => {
                self.master_volume = vol.clamp(0.0, 2.0);
            }
            SynthCommand::SetMelodicProgram {
                channel,
                bank,
                program,
            } => {
                if let Some(ref mut synth) = self.melodic_synth {
                    let ch = channel as i32;
                    // CC 0: Bank Select MSB
                    synth.process_midi_message(ch, 0xB0, 0x00, bank as i32);
                    synth.process_midi_message(ch, 0xC0, program as i32, 0);
                }
            }
            SynthCommand::SetDrumProgram { bank, program } => {
                if let Some(ref mut synth) = self.drum_synth {
                    let ch = Self::PERCUSSION_CHANNEL;
                    synth.process_midi_message(ch, 0xB0, 0x00, bank as i32);
                    synth.process_midi_message(ch, 0xC0, program as i32, 0);
                }
            }
            SynthCommand::SetMacroKnob { knob_index, value } => {
                self.dsp.set_knob(knob_index, value);
            }
            SynthCommand::LooperRecord(track) => {
                self.looper.record_track(track);
            }
            SynthCommand::LooperRecordBoth => {
                self.looper.record_both();
            }
            SynthCommand::LooperOverdub(track) => {
                self.looper.overdub_track(track);
            }
            SynthCommand::LooperPlay => {
                self.looper.play();
            }
            SynthCommand::LooperStop => {
                self.looper.stop();
            }
            SynthCommand::LooperClear(track) => {
                self.looper.clear_track(track);
            }
            SynthCommand::LooperClearAll => {
                self.looper.clear_all();
            }
            SynthCommand::LooperSetBpm(bpm) => {
                self.looper.set_bpm(bpm);
            }
            SynthCommand::LooperSetBars(bars) => {
                self.looper.set_bars(bars);
            }
            SynthCommand::LooperToggleMute(track) => {
                self.looper.toggle_mute(track);
            }
            SynthCommand::LooperSetMetronome(enabled) => {
                self.looper.metronome.set_enabled(enabled);
            }
            SynthCommand::LooperSetMetronomeVolume(vol) => {
                self.looper.metronome.set_volume(vol);
            }
            SynthCommand::LooperSetMetronomeMode(mode) => {
                self.looper.metronome.set_mode(mode);
            }
            SynthCommand::LooperGetMixBuffers(sender) => {
                let (mix_l, mix_r) = self.looper.get_mix_buffers();
                let _ = sender.send((mix_l, mix_r, self.sample_rate as f32));
            }
            SynthCommand::UpdateLoopNote {
                track,
                note_id,
                new_step,
                new_note,
            } => {
                self.looper.update_note(track, &note_id, new_step, new_note);
            }
            SynthCommand::QuantizeLoop { track, grid_step } => {
                self.looper.quantize_track(track, grid_step);
            }
            SynthCommand::DeleteLoopNote { track, note_id } => {
                self.looper.delete_note(track, &note_id);
            }
            SynthCommand::ClearLoopNotes(track) => {
                self.looper.clear_notes(track);
            }
            SynthCommand::Panic => {
                self.panic();
            }
        }
    }

    /// Route incoming performance events to either melodic or drum synth.
    pub fn handle_routed_event(&mut self, event: &RoutedEvent) {
        match event {
            RoutedEvent::KeyboardNoteOn {
                channel,
                note,
                velocity,
            } => {
                self.looper.record_note_on(1, *note, *velocity);
                if let Some(ref mut synth) = self.melodic_synth {
                    // Send to designated channel (remap channel 9 percussion to 0 if routed here)
                    let ch = if *channel == 9 { 0 } else { *channel as i32 };
                    synth.note_on(ch, *note as i32, *velocity as i32);
                }
            }
            RoutedEvent::KeyboardNoteOff { channel, note } => {
                self.looper.record_note_off(1, *note);
                if let Some(ref mut synth) = self.melodic_synth {
                    let ch = if *channel == 9 { 0 } else { *channel as i32 };
                    synth.note_off(ch, *note as i32);
                }
            }
            RoutedEvent::DrumPadOn {
                pad_index: _,
                note,
                velocity,
            } => {
                self.looper.record_note_on(0, *note, *velocity);
                if let Some(ref mut synth) = self.drum_synth {
                    // Triggers percussion voice on standard percussion channel
                    synth.note_on(Self::PERCUSSION_CHANNEL, *note as i32, *velocity as i32);
                }
            }
            RoutedEvent::DrumPadOff {
                pad_index: _,
                note,
            } => {
                self.looper.record_note_off(0, *note);
                if let Some(ref mut synth) = self.drum_synth {
                    synth.note_off(Self::PERCUSSION_CHANNEL, *note as i32);
                }
            }
            RoutedEvent::PitchBend { value } => {
                // value is normalized -1.0 ..= 1.0
                let raw = ((value.clamp(-1.0, 1.0) * 8191.0) as i32 + 8192).clamp(0, 16383);
                let lsb = raw & 0x7F;
                let msb = (raw >> 7) & 0x7F;
                if let Some(ref mut synth) = self.melodic_synth {
                    synth.process_midi_message(0, 0xE0, lsb, msb);
                }
            }
            RoutedEvent::Modulation { value } => {
                // CC 1: value is 0.0 ..= 1.0 normalized
                let cc_val = (value.clamp(0.0, 1.0) * 127.0).round() as i32;
                if let Some(ref mut synth) = self.melodic_synth {
                    synth.process_midi_message(0, 0xB0, 1, cc_val);
                }
            }
            RoutedEvent::SustainPedal { is_down } => {
                // CC 64: Hold Pedal
                let cc_val = if *is_down { 127 } else { 0 };
                if let Some(ref mut synth) = self.melodic_synth {
                    synth.process_midi_message(0, 0xB0, 64, cc_val);
                }
            }
            RoutedEvent::MacroKnob {
                knob_index,
                value,
            } => {
                self.dsp.set_knob(*knob_index, *value);
            }
            RoutedEvent::Raw(midi_event) => match midi_event {
                MidiEvent::ProgramChange { channel, program } => {
                    if let Some(ref mut synth) = self.melodic_synth {
                        synth.process_midi_message(*channel as i32, 0xC0, *program as i32, 0);
                    }
                }
                _ => {}
            },
        }
    }

    /// Silence all active voices immediately across both synthesizers.
    pub fn panic(&mut self) {
        if let Some(ref mut synth) = self.melodic_synth {
            synth.note_off_all(true);
            synth.reset_all_controllers();
        }
        if let Some(ref mut synth) = self.drum_synth {
            synth.note_off_all(true);
            synth.reset_all_controllers();
        }
        self.dsp.reset();
    }

    /// Render a block of stereo frames into separate left and right output slices.
    /// Returns `(peak_left, peak_right, rms_left, rms_right)`.
    pub fn render_stereo(
        &mut self,
        out_left: &mut [f32],
        out_right: &mut [f32],
    ) -> (f32, f32, f32, f32) {
        assert_eq!(
            out_left.len(),
            out_right.len(),
            "Stereo output buffers must have equal length"
        );
        let frames = out_left.len();

        if self.scratch_mel_left.len() < frames {
            self.scratch_mel_left.resize(frames, 0.0);
            self.scratch_mel_right.resize(frames, 0.0);
            self.scratch_drm_left.resize(frames, 0.0);
            self.scratch_drm_right.resize(frames, 0.0);
        }

        // Dispatch sequenced loop notes if transport is playing
        if self.looper.is_running && self.looper.loop_length_samples > 0 {
            let pos = self.looper.playhead;
            let loop_len = self.looper.loop_length_samples;
            let total_steps = self.looper.total_steps() as f32;
            let samples_per_step = loop_len as f32 / total_steps.max(1.0);

            let sample_in_range = |target: usize| -> bool {
                let end = pos + frames;
                if end <= loop_len {
                    target >= pos && target < end
                } else {
                    (target >= pos && target < loop_len) || (target < (end % loop_len))
                }
            };

            // Track 0: Drums
            if self.looper.tracks[0].state == crate::looper::TrackState::Playing
                || self.looper.tracks[0].state == crate::looper::TrackState::Overdubbing
            {
                if let Some(ref mut synth) = self.drum_synth {
                    for note in &self.looper.tracks[0].notes {
                        let start_sample = ((note.start_step * samples_per_step).round() as usize) % loop_len;
                        if sample_in_range(start_sample) {
                            synth.note_on(Self::PERCUSSION_CHANNEL, note.note as i32, note.velocity as i32);
                        }
                        let end_sample = (((note.start_step + note.length_steps) * samples_per_step).round() as usize) % loop_len;
                        if sample_in_range(end_sample) {
                            synth.note_off(Self::PERCUSSION_CHANNEL, note.note as i32);
                        }
                    }
                }
            }

            // Track 1: Keys
            if self.looper.tracks[1].state == crate::looper::TrackState::Playing
                || self.looper.tracks[1].state == crate::looper::TrackState::Overdubbing
            {
                if let Some(ref mut synth) = self.melodic_synth {
                    for note in &self.looper.tracks[1].notes {
                        let start_sample = ((note.start_step * samples_per_step).round() as usize) % loop_len;
                        if sample_in_range(start_sample) {
                            synth.note_on(0, note.note as i32, note.velocity as i32);
                        }
                        let end_sample = (((note.start_step + note.length_steps) * samples_per_step).round() as usize) % loop_len;
                        if sample_in_range(end_sample) {
                            synth.note_off(0, note.note as i32);
                        }
                    }
                }
            }
        }

        // Render melodic synth
        let (mel_l, mel_r) = (
            &mut self.scratch_mel_left[..frames],
            &mut self.scratch_mel_right[..frames],
        );
        if let Some(ref mut synth) = self.melodic_synth {
            synth.render(mel_l, mel_r);
        } else {
            mel_l.fill(0.0);
            mel_r.fill(0.0);
        }

        // Render drum kit synth
        let (drm_l, drm_r) = (
            &mut self.scratch_drm_left[..frames],
            &mut self.scratch_drm_right[..frames],
        );
        if let Some(ref mut synth) = self.drum_synth {
            synth.render(drm_l, drm_r);
        } else {
            drm_l.fill(0.0);
            drm_r.fill(0.0);
        }

        let (keys_gain, drums_gain) = self.dsp.get_keys_drums_gains();
        let mel_vol = self.melodic_volume * keys_gain;
        let drm_vol = self.drum_volume * drums_gain;

        for i in 0..frames {
            self.scratch_mel_left[i] *= mel_vol;
            self.scratch_mel_right[i] *= mel_vol;
            self.scratch_drm_left[i] *= drm_vol;
            self.scratch_drm_right[i] *= drm_vol;

            out_left[i] = self.scratch_mel_left[i] + self.scratch_drm_left[i];
            out_right[i] = self.scratch_mel_right[i] + self.scratch_drm_right[i];
        }

        // Process through DSP chain (SVF Filter -> Chorus -> Reverb -> Master Gain -> Soft Saturator)
        self.dsp.process_stereo(out_left, out_right);

        // Process through 2-Track Looper Engine with isolated inputs:
        // Track 0 (Drums) strictly receives drum audio only.
        // Track 1 (Piano/Keys) strictly receives piano audio only.
        self.looper.process_isolated(
            &self.scratch_drm_left[..frames],
            &self.scratch_drm_right[..frames],
            &self.scratch_mel_left[..frames],
            &self.scratch_mel_right[..frames],
            out_left,
            out_right,
        );

        let mut peak_left = 0.0f32;
        let mut peak_right = 0.0f32;
        let mut sum_sq_left = 0.0f32;
        let mut sum_sq_right = 0.0f32;

        for i in 0..frames {
            let l = out_left[i];
            let r = out_right[i];

            let abs_l = l.abs();
            let abs_r = r.abs();
            if abs_l > peak_left {
                peak_left = abs_l;
            }
            if abs_r > peak_right {
                peak_right = abs_r;
            }
            sum_sq_left += l * l;
            sum_sq_right += r * r;
        }

        let rms_left = if frames > 0 {
            (sum_sq_left / frames as f32).sqrt()
        } else {
            0.0
        };
        let rms_right = if frames > 0 {
            (sum_sq_right / frames as f32).sqrt()
        } else {
            0.0
        };

        (peak_left, peak_right, rms_left, rms_right)
    }

    /// Render a block of audio directly into an interleaved output slice (e.g. for CPAL stream callback).
    /// Directly delegates to `render_stereo` to ensure isolated track processing (`process_isolated`)
    /// and scheduled loop notes are always active in real-time hardware playback.
    /// Returns `(peak_left, peak_right, rms_left, rms_right)`.
    pub fn render_interleaved(
        &mut self,
        out_interleaved: &mut [f32],
        channels: usize,
    ) -> (f32, f32, f32, f32) {
        if channels == 0 || out_interleaved.is_empty() {
            return (0.0, 0.0, 0.0, 0.0);
        }

        let frames = out_interleaved.len() / channels;
        let mut scratch_l = std::mem::take(&mut self.scratch_out_left);
        let mut scratch_r = std::mem::take(&mut self.scratch_out_right);
        if scratch_l.len() < frames {
            scratch_l.resize(frames, 0.0);
            scratch_r.resize(frames, 0.0);
        }

        let (peak_left, peak_right, rms_left, rms_right) = self.render_stereo(
            &mut scratch_l[..frames],
            &mut scratch_r[..frames],
        );

        for i in 0..frames {
            let l = scratch_l[i];
            let r = scratch_r[i];

            if channels == 2 {
                out_interleaved[i * 2] = l;
                out_interleaved[i * 2 + 1] = r;
            } else if channels == 1 {
                out_interleaved[i] = (l + r) * 0.5;
            } else {
                out_interleaved[i * channels] = l;
                out_interleaved[i * channels + 1] = r;
                for c in 2..channels {
                    out_interleaved[i * channels + c] = 0.0;
                }
            }
        }

        self.scratch_out_left = scratch_l;
        self.scratch_out_right = scratch_r;

        (peak_left, peak_right, rms_left, rms_right)
    }
}

/// Helper to instantiate a pure-Rust SoundFont Synthesizer with optimal settings.
pub fn create_synthesizer(
    soundfont: &Arc<SoundFont>,
    sample_rate: i32,
) -> Result<Synthesizer, SynthesizerError> {
    let settings = SynthesizerSettings::new(sample_rate);
    Synthesizer::new(soundfont, &settings)
}

/// Load a SoundFont from an arbitrary `Read` stream into an `Arc<SoundFont>`.
pub fn load_soundfont_from_reader<R: Read>(reader: &mut R) -> Result<Arc<SoundFont>, SoundFontError> {
    let soundfont = SoundFont::new(reader)?;
    Ok(Arc::new(soundfont))
}

/// Load a SoundFont from an in-memory byte slice into an `Arc<SoundFont>`.
pub fn load_soundfont_from_bytes(bytes: &[u8]) -> Result<Arc<SoundFont>, SoundFontError> {
    let mut cursor = Cursor::new(bytes);
    load_soundfont_from_reader(&mut cursor)
}

/// Load a SoundFont from a file path into an `Arc<SoundFont>`.
pub fn load_soundfont_from_path<P: AsRef<Path>>(
    path: P,
) -> Result<Arc<SoundFont>, Box<dyn std::error::Error + Send + Sync>> {
    let mut file = File::open(path)?;
    let soundfont = SoundFont::new(&mut file)?;
    Ok(Arc::new(soundfont))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dual_synth_initial_state() {
        let mut engine = DualSynthEngine::new(44100);
        assert_eq!(engine.has_melodic_synth(), false);
        assert_eq!(engine.has_drum_synth(), false);
        assert_eq!(engine.melodic_volume(), 1.0);
        assert_eq!(engine.drum_volume(), 1.0);
        assert_eq!(engine.master_volume(), 0.9);

        // Rendering with no active soundfonts outputs silence cleanly
        let mut left = vec![1.0; 256];
        let mut right = vec![1.0; 256];
        let (peak_l, peak_r, rms_l, rms_r) = engine.render_stereo(&mut left, &mut right);
        assert_eq!(peak_l, 0.0);
        assert_eq!(peak_r, 0.0);
        assert_eq!(rms_l, 0.0);
        assert_eq!(rms_r, 0.0);
        assert!(left.iter().all(|&x| x == 0.0));
        assert!(right.iter().all(|&x| x == 0.0));
    }

    #[test]
    fn test_dual_synth_commands() {
        let mut engine = DualSynthEngine::new(48000);
        engine.handle_command(SynthCommand::SetMelodicVolume(0.75));
        engine.handle_command(SynthCommand::SetDrumVolume(0.85));
        engine.handle_command(SynthCommand::SetMasterVolume(0.95));

        assert!((engine.melodic_volume() - 0.75).abs() < 1e-6);
        assert!((engine.drum_volume() - 0.85).abs() < 1e-6);
        assert!((engine.master_volume() - 0.95).abs() < 1e-6);
    }

    #[test]
    fn test_interleaved_rendering_silence() {
        let mut engine = DualSynthEngine::new(44100);
        let mut buffer = vec![0.5f32; 512];
        let (peak_l, peak_r, _, _) = engine.render_interleaved(&mut buffer, 2);

        assert_eq!(peak_l, 0.0);
        assert_eq!(peak_r, 0.0);
        assert!(buffer.iter().all(|&x| x == 0.0));
    }

    #[test]
    fn test_audio_meter_atomic_updates() {
        let meter = AudioMeter::new();
        let (pl, pr, rl, rr) = meter.get_levels();
        assert_eq!((pl, pr, rl, rr), (0.0, 0.0, 0.0, 0.0));

        meter.update(0.8, 0.75, 0.45, 0.42);
        let (pl, pr, rl, rr) = meter.get_levels();
        assert!((pl - 0.8).abs() < 1e-6);
        assert!((pr - 0.75).abs() < 1e-6);
        assert!((rl - 0.45).abs() < 1e-6);
        assert!((rr - 0.42).abs() < 1e-6);
    }

    #[test]
    fn test_event_routing_without_panic() {
        let mut engine = DualSynthEngine::new(44100);

        // Feed various events when no synth is loaded; verify none crash or panic
        engine.handle_routed_event(&RoutedEvent::KeyboardNoteOn {
            channel: 0,
            note: 60,
            velocity: 100,
        });
        engine.handle_routed_event(&RoutedEvent::KeyboardNoteOff {
            channel: 0,
            note: 60,
        });
        engine.handle_routed_event(&RoutedEvent::DrumPadOn {
            pad_index: 0,
            note: 36,
            velocity: 110,
        });
        engine.handle_routed_event(&RoutedEvent::DrumPadOff {
            pad_index: 0,
            note: 36,
        });
        engine.handle_routed_event(&RoutedEvent::PitchBend { value: 0.5 });
        engine.handle_routed_event(&RoutedEvent::Modulation { value: 0.8 });
        engine.handle_routed_event(&RoutedEvent::SustainPedal { is_down: true });
        engine.panic();
    }

    #[test]
    fn test_dual_synth_hot_swap_independence() {
        let mut engine = DualSynthEngine::new(44100);
        // Both empty
        assert_eq!(engine.has_melodic_synth(), false);
        assert_eq!(engine.has_drum_synth(), false);

        // Clearing melodic when drum is active does not affect drum synth
        engine.handle_command(SynthCommand::ClearMelodicSynth);
        assert_eq!(engine.has_melodic_synth(), false);

        engine.handle_command(SynthCommand::ClearDrumSynth);
        assert_eq!(engine.has_drum_synth(), false);
    }

    #[test]
    fn test_synth_looper_commands() {
        let mut engine = DualSynthEngine::new(44100);
        assert_eq!(engine.looper.is_running, false);

        engine.handle_command(SynthCommand::LooperSetBpm(130.0));
        assert_eq!(engine.looper.bpm, 130.0);

        engine.handle_command(SynthCommand::LooperSetBars(2));
        assert_eq!(engine.looper.bar_count, 2);

        engine.handle_command(SynthCommand::LooperRecord(0));
        assert_eq!(engine.looper.is_running, true);
        assert_eq!(engine.looper.tracks[0].state, crate::looper::TrackState::Recording);

        engine.handle_command(SynthCommand::LooperStop);
        assert_eq!(engine.looper.is_running, false);

        let (tx, rx) = crossbeam_channel::bounded(1);
        engine.handle_command(SynthCommand::LooperGetMixBuffers(tx));
        let (mix_l, mix_r, sr) = rx.recv().unwrap();
        assert_eq!(sr, 44100.0);
        assert_eq!(mix_l.len(), mix_r.len());
    }
}
