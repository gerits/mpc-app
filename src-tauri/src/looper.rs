use std::collections::HashMap;
use serde::{Deserialize, Serialize};

use crate::metronome::{Metronome, MetronomeMode};
use crate::types::LoopNote;

/// Operating state of an individual looper track.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TrackState {
    Empty,
    Recording,
    Playing,
    Overdubbing,
    Muted,
}

/// A single stereo loop track buffer with recorded audio and sequenced notes.
#[derive(Debug, Clone)]
pub struct LooperTrack {
    pub state: TrackState,
    pub buffer_l: Vec<f32>,
    pub buffer_r: Vec<f32>,
    pub volume: f32,
    pub has_audio: bool,
    pub notes: Vec<LoopNote>,
    pub active_notes: HashMap<u8, (f32, u8, String)>,
    pub waveform_peaks: Vec<f32>,
}

impl LooperTrack {
    pub fn new(capacity: usize) -> Self {
        Self {
            state: TrackState::Empty,
            buffer_l: vec![0.0; capacity],
            buffer_r: vec![0.0; capacity],
            volume: 1.0,
            has_audio: false,
            notes: Vec::new(),
            active_notes: HashMap::new(),
            waveform_peaks: vec![0.0; 64],
        }
    }

    pub fn resize(&mut self, new_capacity: usize) {
        self.buffer_l.resize(new_capacity, 0.0);
        self.buffer_r.resize(new_capacity, 0.0);
    }

    pub fn clear(&mut self) {
        self.buffer_l.fill(0.0);
        self.buffer_r.fill(0.0);
        self.state = TrackState::Empty;
        self.has_audio = false;
        self.notes.clear();
        self.active_notes.clear();
        self.waveform_peaks.fill(0.0);
    }
}

/// Instantaneous status snapshot for broadcasting to the React UI transport bar.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LooperSnapshot {
    pub bpm: f32,
    pub bar_count: u32,
    pub current_bar: u32,
    pub current_beat: u32,
    pub progress: f32,
    pub is_running: bool,
    pub track1_state: TrackState,
    pub track2_state: TrackState,
    pub track1_has_audio: bool,
    pub track2_has_audio: bool,
    #[serde(default)]
    pub metronome_enabled: bool,
    #[serde(default)]
    pub metronome_volume: f32,
    #[serde(default)]
    pub metronome_mode: MetronomeMode,
    #[serde(default)]
    pub metronome_is_ticking: bool,
    #[serde(default)]
    pub track1_notes: Vec<LoopNote>,
    #[serde(default)]
    pub track2_notes: Vec<LoopNote>,
    #[serde(default)]
    pub waveform_peaks_t1: Vec<f32>,
    #[serde(default)]
    pub waveform_peaks_t2: Vec<f32>,
}

impl Default for LooperSnapshot {
    fn default() -> Self {
        Self {
            bpm: LooperEngine::DEFAULT_BPM,
            bar_count: LooperEngine::DEFAULT_BARS,
            current_bar: 1,
            current_beat: 1,
            progress: 0.0,
            is_running: false,
            track1_state: TrackState::Empty,
            track2_state: TrackState::Empty,
            track1_has_audio: false,
            track2_has_audio: false,
            metronome_enabled: false,
            metronome_volume: Metronome::DEFAULT_VOLUME,
            metronome_mode: MetronomeMode::WhileRunning,
            metronome_is_ticking: false,
            track1_notes: Vec::new(),
            track2_notes: Vec::new(),
            waveform_peaks_t1: Vec::new(),
            waveform_peaks_t2: Vec::new(),
        }
    }
}

/// Realtime 2-Track Audio Looper Engine with bar quantization and seamless boundary wrap.
pub struct LooperEngine {
    pub sample_rate: f32,
    pub bpm: f32,
    pub beats_per_bar: u32,
    pub bar_count: u32,
    pub loop_length_samples: usize,
    pub playhead: usize,
    pub is_running: bool,
    pub tracks: [LooperTrack; 2],
    pub overdub_decay: f32,
    pub metronome: Metronome,
}

impl LooperEngine {
    pub const DEFAULT_BPM: f32 = 120.0;
    pub const DEFAULT_BARS: u32 = 4;
    pub const DEFAULT_BEATS: u32 = 4;

    pub fn new(sample_rate: f32) -> Self {
        let sr = sample_rate.max(1000.0);
        let bpm = Self::DEFAULT_BPM;
        let bars = Self::DEFAULT_BARS;
        let beats = Self::DEFAULT_BEATS;

        let loop_len = Self::calculate_loop_samples(sr, bpm, beats, bars);
        let mut metronome = Metronome::new(sr);
        metronome.set_bpm(bpm);
        metronome.set_beats_per_bar(beats);

        Self {
            sample_rate: sr,
            bpm,
            beats_per_bar: beats,
            bar_count: bars,
            loop_length_samples: loop_len,
            playhead: 0,
            is_running: false,
            tracks: [LooperTrack::new(loop_len), LooperTrack::new(loop_len)],
            overdub_decay: 0.95,
            metronome,
        }
    }

    pub fn calculate_loop_samples(sample_rate: f32, bpm: f32, beats_per_bar: u32, bar_count: u32) -> usize {
        let samples_per_beat = (sample_rate * 60.0) / bpm.clamp(40.0, 300.0);
        let samples_per_bar = samples_per_beat * (beats_per_bar.max(1) as f32);
        (samples_per_bar * (bar_count.max(1) as f32)).round() as usize
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate.max(1000.0);
        self.metronome.set_sample_rate(self.sample_rate);
        self.recalculate_length();
    }

    pub fn set_bpm(&mut self, bpm: f32) {
        self.bpm = bpm.clamp(40.0, 300.0);
        self.metronome.set_bpm(self.bpm);
        self.recalculate_length();
    }

    pub fn set_bars(&mut self, bar_count: u32) {
        self.bar_count = bar_count.clamp(1, 16);
        self.recalculate_length();
    }

    fn recalculate_length(&mut self) {
        let new_len = Self::calculate_loop_samples(
            self.sample_rate,
            self.bpm,
            self.beats_per_bar,
            self.bar_count,
        );
        self.loop_length_samples = new_len;
        for track in &mut self.tracks {
            track.resize(new_len);
        }
        if self.playhead >= new_len {
            self.playhead = 0;
        }
    }

    /// Arm or toggle Recording on a specified track (0 or 1).
    pub fn record_track(&mut self, track_idx: usize) {
        if track_idx > 1 {
            return;
        }

        let state = self.tracks[track_idx].state;
        match state {
            TrackState::Empty | TrackState::Playing | TrackState::Muted => {
                // Clear existing track notes and waveform peaks for fresh recording pass
                // Audio buffer is overwritten sample-by-sample in real time, avoiding heavy multi-megabyte memsets in RT thread
                self.tracks[track_idx].notes.clear();
                self.tracks[track_idx].active_notes.clear();
                self.tracks[track_idx].waveform_peaks.fill(0.0);
                self.tracks[track_idx].state = TrackState::Recording;
                self.tracks[track_idx].has_audio = true;
                // If looper was stopped, reset playhead to beat 1 and start running
                if !self.is_running {
                    self.playhead = 0;
                    self.is_running = true;
                }
            }
            TrackState::Recording => {
                // User pressed Record again to end take early and loop
                self.tracks[track_idx].state = TrackState::Playing;
            }
            TrackState::Overdubbing => {
                self.tracks[track_idx].state = TrackState::Playing;
            }
        }
    }

    /// Arm or toggle recording on BOTH tracks simultaneously for full 2-track live jamming.
    pub fn record_both(&mut self) {
        let both_recording = self.tracks[0].state == TrackState::Recording
            && self.tracks[1].state == TrackState::Recording;
        if both_recording {
            self.play();
        } else {
            self.record_track(0);
            self.record_track(1);
        }
    }

    /// Arm or toggle Overdubbing on a specified track (0 or 1).
    pub fn overdub_track(&mut self, track_idx: usize) {
        if track_idx > 1 {
            return;
        }

        match self.tracks[track_idx].state {
            TrackState::Playing | TrackState::Muted => {
                self.tracks[track_idx].state = TrackState::Overdubbing;
                self.is_running = true;
            }
            TrackState::Overdubbing => {
                self.tracks[track_idx].state = TrackState::Playing;
            }
            TrackState::Empty => {
                // If empty, initiate first recording pass
                self.record_track(track_idx);
            }
            TrackState::Recording => {
                self.tracks[track_idx].state = TrackState::Overdubbing;
            }
        }
    }

    /// Start or resume transport playback.
    pub fn play(&mut self) {
        self.is_running = true;
        self.metronome.reset_standalone_phase();
        let loop_len = self.loop_length_samples;
        for track in &mut self.tracks {
            if track.state == TrackState::Recording {
                track.state = TrackState::Playing;
                Self::apply_seam_smoothing(&mut track.buffer_l, &mut track.buffer_r, loop_len);
            }
        }
    }

    /// Stop transport playback and rewind playhead to beginning.
    pub fn stop(&mut self) {
        self.is_running = false;
        self.playhead = 0;
        self.metronome.reset_standalone_phase();

        for track in &mut self.tracks {
            if track.state == TrackState::Recording || track.state == TrackState::Overdubbing {
                if track.has_audio {
                    track.state = TrackState::Playing;
                } else {
                    track.state = TrackState::Empty;
                }
            }
        }
    }

    /// Toggle mute on a track.
    pub fn toggle_mute(&mut self, track_idx: usize) {
        if track_idx > 1 {
            return;
        }

        match self.tracks[track_idx].state {
            TrackState::Playing => self.tracks[track_idx].state = TrackState::Muted,
            TrackState::Muted => self.tracks[track_idx].state = TrackState::Playing,
            _ => {}
        }
    }

    /// Clear recorded buffer of an individual track.
    pub fn clear_track(&mut self, track_idx: usize) {
        if track_idx <= 1 {
            self.tracks[track_idx].clear();
        }
    }

    /// Clear all tracks and reset transport.
    pub fn clear_all(&mut self) {
        for track in &mut self.tracks {
            track.clear();
        }
        self.playhead = 0;
        self.is_running = false;
        self.metronome.reset_standalone_phase();
    }

    /// Total 16th-note steps in the loop (e.g. 4 bars * 4 beats * 4 = 64).
    pub fn total_steps(&self) -> u32 {
        self.bar_count * self.beats_per_bar * 4
    }

    /// Current playhead position in fractional 16th-note steps.
    pub fn current_step(&self) -> f32 {
        if self.loop_length_samples == 0 {
            0.0
        } else {
            (self.playhead as f32 / self.loop_length_samples as f32) * (self.total_steps() as f32)
        }
    }

    /// Record a note-on event for track (0 or 1) if track is recording or overdubbing.
    pub fn record_note_on(&mut self, track_idx: usize, note: u8, velocity: u8) {
        if track_idx > 1 {
            return;
        }
        if self.tracks[track_idx].state != TrackState::Recording
            && self.tracks[track_idx].state != TrackState::Overdubbing
        {
            return;
        }
        let start_step = self.current_step();
        let track = &mut self.tracks[track_idx];
        let id = format!("t{}-{}-{}", track_idx, note, track.notes.len() + track.active_notes.len());
        track.active_notes.insert(note, (start_step, velocity, id));
    }

    /// Record a note-off event for track (0 or 1).
    pub fn record_note_off(&mut self, track_idx: usize, note: u8) {
        if track_idx > 1 {
            return;
        }
        let total = self.total_steps() as f32;
        let current_step = self.current_step();
        let track = &mut self.tracks[track_idx];
        if let Some((start_step, velocity, id)) = track.active_notes.remove(&note) {
            let mut length_steps = current_step - start_step;
            if length_steps <= 0.0 {
                length_steps += total;
            }
            let min_len = if track_idx == 0 { 1.0 } else { 0.25 };
            length_steps = length_steps.clamp(min_len, total);
            let diff = (start_step - start_step.round()).abs();
            let off_beat = diff > 0.08;
            track.notes.push(LoopNote {
                id,
                track_index: track_idx,
                note,
                velocity,
                start_step,
                length_steps,
                off_beat,
            });
        }
    }

    /// Update note position and/or pitch when dragged by the user in the UI.
    pub fn update_note(&mut self, track_idx: usize, note_id: &str, new_step: f32, new_note: u8) {
        if track_idx > 1 {
            return;
        }
        let total = self.total_steps() as f32;
        if let Some(note) = self.tracks[track_idx].notes.iter_mut().find(|n| n.id == note_id) {
            note.start_step = new_step.clamp(0.0, (total - 0.01).max(0.0));
            note.note = new_note;
            let diff = (note.start_step - note.start_step.round()).abs();
            note.off_beat = diff > 0.08;
        }
    }

    /// Quantize all notes on track to nearest grid step (e.g. 1.0 = 1/16th, 2.0 = 1/8th).
    pub fn quantize_track(&mut self, track_idx: usize, grid_step: f32) {
        if track_idx > 1 {
            return;
        }
        let step = grid_step.max(0.25);
        let total = self.total_steps() as f32;
        for note in &mut self.tracks[track_idx].notes {
            note.start_step = ((note.start_step / step).round() * step) % total;
            note.off_beat = false;
        }
    }

    /// Delete a single note by ID.
    pub fn delete_note(&mut self, track_idx: usize, note_id: &str) {
        if track_idx <= 1 {
            self.tracks[track_idx].notes.retain(|n| n.id != note_id);
        }
    }

    /// Clear all notes on a track.
    pub fn clear_notes(&mut self, track_idx: usize) {
        if track_idx <= 1 {
            self.tracks[track_idx].notes.clear();
            self.tracks[track_idx].active_notes.clear();
        }
    }

    /// Compute downsampled peak envelope from audio buffer for UI waveform visualization.
    pub fn compute_waveform_peaks(buf_l: &[f32], buf_r: &[f32], len: usize, bins: usize) -> Vec<f32> {
        if len == 0 || bins == 0 {
            return Vec::new();
        }
        let chunk_size = (len / bins).max(1);
        let mut peaks = Vec::with_capacity(bins);
        for b in 0..bins {
            let start = b * chunk_size;
            let end = ((b + 1) * chunk_size).min(len);
            if start >= len {
                peaks.push(0.0);
                continue;
            }
            let mut max_val = 0.0f32;
            for i in start..end {
                let l = buf_l[i].abs();
                let r = buf_r[i].abs();
                let m = l.max(r);
                if m > max_val {
                    max_val = m;
                }
            }
            peaks.push(max_val.min(1.0));
        }
        peaks
    }

    /// Query the current progress snapshot.
    pub fn get_snapshot(&self) -> LooperSnapshot {
        let total_samples = self.loop_length_samples.max(1);
        let progress = (self.playhead as f32 / total_samples as f32).clamp(0.0, 1.0);

        let samples_per_bar = total_samples as f32 / self.bar_count.max(1) as f32;
        let current_bar = ((self.playhead as f32 / samples_per_bar).floor() as u32 + 1).min(self.bar_count);

        let samples_per_beat = samples_per_bar / self.beats_per_bar.max(1) as f32;
        let beat_within_bar = ((self.playhead as f32 % samples_per_bar) / samples_per_beat).floor() as u32 + 1;
        let current_beat = beat_within_bar.min(self.beats_per_bar);
        let is_ticking = self.metronome.is_ticking(self.is_running);

        let peaks_t1 = self.tracks[0].waveform_peaks.clone();
        let peaks_t2 = self.tracks[1].waveform_peaks.clone();

        LooperSnapshot {
            bpm: self.bpm,
            bar_count: self.bar_count,
            current_bar,
            current_beat,
            progress,
            is_running: self.is_running,
            track1_state: self.tracks[0].state,
            track2_state: self.tracks[1].state,
            track1_has_audio: self.tracks[0].has_audio,
            track2_has_audio: self.tracks[1].has_audio,
            metronome_enabled: self.metronome.enabled,
            metronome_volume: self.metronome.volume,
            metronome_mode: self.metronome.mode,
            metronome_is_ticking: is_ticking,
            track1_notes: self.tracks[0].notes.clone(),
            track2_notes: self.tracks[1].notes.clone(),
            waveform_peaks_t1: peaks_t1,
            waveform_peaks_t2: peaks_t2,
        }
    }

    /// Read track buffers for audio processing or WAV export.
    pub fn get_track_buffers(&self, track_idx: usize) -> Option<(&[f32], &[f32])> {
        if track_idx > 1 {
            return None;
        }
        let len = self.loop_length_samples;
        Some((&self.tracks[track_idx].buffer_l[..len], &self.tracks[track_idx].buffer_r[..len]))
    }

    /// Sums both tracks into a combined stereo mix for WAV export.
    pub fn get_mix_buffers(&self) -> (Vec<f32>, Vec<f32>) {
        let len = self.loop_length_samples;
        let mut mix_l = vec![0.0; len];
        let mut mix_r = vec![0.0; len];

        for track in &self.tracks {
            if track.has_audio && track.state != TrackState::Muted {
                let vol = track.volume;
                for i in 0..len {
                    mix_l[i] += track.buffer_l[i] * vol;
                    mix_r[i] += track.buffer_r[i] * vol;
                }
            }
        }

        // Soft limit to prevent digital clipping in exported file
        for i in 0..len {
            mix_l[i] = mix_l[i].clamp(-1.0, 1.0);
            mix_r[i] = mix_r[i].clamp(-1.0, 1.0);
        }

        (mix_l, mix_r)
    }

    /// Isolated processing callback routing dedicated audio streams to Track 0 (Drums) and Track 1 (Piano/Keys).
    /// Playing piano will NEVER be captured into Track 0 (Drums) even when played simultaneously.
    pub fn process_isolated(
        &mut self,
        drums_l: &[f32],
        drums_r: &[f32],
        keys_l: &[f32],
        keys_r: &[f32],
        out_l: &mut [f32],
        out_r: &mut [f32],
    ) {
        assert_eq!(out_l.len(), out_r.len());
        if !self.is_running {
            if self.metronome.enabled && self.metronome.mode == MetronomeMode::Always {
                self.metronome.process_stereo_stopped(out_l, out_r);
            }
            return;
        }

        if self.loop_length_samples == 0 {
            return;
        }

        let frames = out_l.len();
        let loop_len = self.loop_length_samples;
        let decay = self.overdub_decay;
        let total_beats = self.bar_count * self.beats_per_bar;
        let beats_per_bar = self.beats_per_bar;
        let samples_per_beat = if total_beats > 0 { loop_len as f64 / total_beats as f64 } else { 0.0 };

        for i in 0..frames {
            let pos = self.playhead;
            let bin = if loop_len > 0 { (pos * 64) / loop_len } else { 0 };

            // 1. Process Track 0 (Drums): strictly receives ONLY drums_l and drums_r
            let (d_l, d_r) = (drums_l[i], drums_r[i]);
            match self.tracks[0].state {
                TrackState::Recording => {
                    self.tracks[0].buffer_l[pos] = d_l;
                    self.tracks[0].buffer_r[pos] = d_r;
                    if bin < 64 {
                        let p = d_l.abs().max(d_r.abs()).min(1.0);
                        if p > self.tracks[0].waveform_peaks[bin] {
                            self.tracks[0].waveform_peaks[bin] = p;
                        }
                    }
                }
                TrackState::Overdubbing => {
                    let ex_l = self.tracks[0].buffer_l[pos];
                    let ex_r = self.tracks[0].buffer_r[pos];
                    let new_l = ex_l * decay + d_l;
                    let new_r = ex_r * decay + d_r;
                    self.tracks[0].buffer_l[pos] = new_l;
                    self.tracks[0].buffer_r[pos] = new_r;
                    if bin < 64 {
                        let p = new_l.abs().max(new_r.abs()).min(1.0);
                        if p > self.tracks[0].waveform_peaks[bin] {
                            self.tracks[0].waveform_peaks[bin] = p;
                        }
                    }
                    if self.tracks[0].notes.is_empty() {
                        out_l[i] += ex_l * self.tracks[0].volume;
                        out_r[i] += ex_r * self.tracks[0].volume;
                    }
                }
                TrackState::Playing => {
                    if self.tracks[0].notes.is_empty() {
                        out_l[i] += self.tracks[0].buffer_l[pos] * self.tracks[0].volume;
                        out_r[i] += self.tracks[0].buffer_r[pos] * self.tracks[0].volume;
                    }
                }
                TrackState::Muted | TrackState::Empty => {}
            }

            // 2. Process Track 1 (Piano / Keys): strictly receives ONLY keys_l and keys_r
            let (k_l, k_r) = (keys_l[i], keys_r[i]);
            match self.tracks[1].state {
                TrackState::Recording => {
                    self.tracks[1].buffer_l[pos] = k_l;
                    self.tracks[1].buffer_r[pos] = k_r;
                    if bin < 64 {
                        let p = k_l.abs().max(k_r.abs()).min(1.0);
                        if p > self.tracks[1].waveform_peaks[bin] {
                            self.tracks[1].waveform_peaks[bin] = p;
                        }
                    }
                }
                TrackState::Overdubbing => {
                    let ex_l = self.tracks[1].buffer_l[pos];
                    let ex_r = self.tracks[1].buffer_r[pos];
                    let new_l = ex_l * decay + k_l;
                    let new_r = ex_r * decay + k_r;
                    self.tracks[1].buffer_l[pos] = new_l;
                    self.tracks[1].buffer_r[pos] = new_r;
                    if bin < 64 {
                        let p = new_l.abs().max(new_r.abs()).min(1.0);
                        if p > self.tracks[1].waveform_peaks[bin] {
                            self.tracks[1].waveform_peaks[bin] = p;
                        }
                    }
                    if self.tracks[1].notes.is_empty() {
                        out_l[i] += ex_l * self.tracks[1].volume;
                        out_r[i] += ex_r * self.tracks[1].volume;
                    }
                }
                TrackState::Playing => {
                    if self.tracks[1].notes.is_empty() {
                        out_l[i] += self.tracks[1].buffer_l[pos] * self.tracks[1].volume;
                        out_r[i] += self.tracks[1].buffer_r[pos] * self.tracks[1].volume;
                    }
                }
                TrackState::Muted | TrackState::Empty => {}
            }

            // 3. Synthesize metronome click into monitor output ONLY.
            let metro_click = self.metronome.process_sample_running(
                pos,
                samples_per_beat,
                total_beats,
                beats_per_bar,
            );
            out_l[i] += metro_click;
            out_r[i] += metro_click;

            // 4. Advance playhead
            self.playhead += 1;
            if self.playhead >= loop_len {
                self.playhead = 0;

                // Seamless loop transition: any track finishing its first record pass loops automatically
                for (t_idx, track) in self.tracks.iter_mut().enumerate() {
                    if track.state == TrackState::Recording {
                        track.state = TrackState::Playing;
                        // Finalize any active notes still awaiting note off
                        let total = (self.bar_count * self.beats_per_bar * 4) as f32;
                        let active: Vec<_> = track.active_notes.drain().collect();
                        for (note, (start_step, velocity, id)) in active {
                            let mut length_steps = total - start_step;
                            if length_steps <= 0.0 { length_steps = 1.0; }
                            let off_beat = (start_step - start_step.round()).abs() > 0.08;
                            track.notes.push(LoopNote {
                                id,
                                track_index: t_idx,
                                note,
                                velocity,
                                start_step,
                                length_steps,
                                off_beat,
                            });
                        }
                        // Apply micro crossfade across 32 samples at the loop seam
                        Self::apply_seam_smoothing(&mut track.buffer_l, &mut track.buffer_r, loop_len);
                    }
                }
            }
        }
    }

    /// Realtime processing callback fallback inside the audio engine thread.
    pub fn process_stereo(&mut self, left: &mut [f32], right: &mut [f32]) {
        let in_l = left.to_vec();
        let in_r = right.to_vec();
        self.process_isolated(&in_l, &in_r, &in_l, &in_r, left, right);
    }

    /// Smooth loop start and end boundaries with a micro-fade to eliminate turnaround clicks.
    fn apply_seam_smoothing(buf_l: &mut [f32], buf_r: &mut [f32], len: usize) {
        const FADE_LEN: usize = 32;
        if len < FADE_LEN * 2 {
            return;
        }

        for i in 0..FADE_LEN {
            let gain = i as f32 / FADE_LEN as f32;
            buf_l[i] *= gain;
            buf_r[i] *= gain;

            let end_idx = len - 1 - i;
            buf_l[end_idx] *= gain;
            buf_r[end_idx] *= gain;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_looper_sample_calculation() {
        // 120 BPM, 4 beats/bar, 4 bars at 44100 Hz
        // 1 beat = 0.5s = 22050 samples
        // 4 beats = 1 bar = 88200 samples
        // 4 bars = 352800 samples
        let samples = LooperEngine::calculate_loop_samples(44100.0, 120.0, 4, 4);
        assert_eq!(samples, 352800);
    }

    #[test]
    fn test_looper_record_and_playback_transition() {
        let mut looper = LooperEngine::new(44100.0);
        looper.set_bpm(120.0);
        looper.set_bars(1); // 1 bar = 88200 samples

        // Arm track 0 for recording
        looper.record_track(0);
        assert_eq!(looper.tracks[0].state, TrackState::Recording);
        assert!(looper.is_running);

        let block_size = 512;
        let mut left = vec![0.8f32; block_size];
        let mut right = vec![0.8f32; block_size];

        // Process audio for 1 complete bar
        let iterations = looper.loop_length_samples / block_size + 1;
        for _ in 0..iterations {
            left.fill(0.8);
            right.fill(0.8);
            looper.process_stereo(&mut left, &mut right);
        }

        // Track 0 must have automatically transitioned from Recording to Playing!
        assert_eq!(looper.tracks[0].state, TrackState::Playing);
        assert!(looper.tracks[0].has_audio);
    }

    #[test]
    fn test_looper_overdub() {
        let mut looper = LooperEngine::new(44100.0);
        looper.set_bars(1);

        // Record initial pass
        looper.record_track(0);
        let mut left = vec![0.5f32; looper.loop_length_samples];
        let mut right = vec![0.5f32; looper.loop_length_samples];
        looper.process_stereo(&mut left, &mut right);
        assert_eq!(looper.tracks[0].state, TrackState::Playing);

        // Arm overdub
        looper.overdub_track(0);
        assert_eq!(looper.tracks[0].state, TrackState::Overdubbing);

        // Overdub another layer
        left.fill(0.2);
        right.fill(0.2);
        looper.process_stereo(&mut left, &mut right);

        // Buffer must now contain layered sound
        let buf = &looper.tracks[0].buffer_l;
        assert!(buf[100] > 0.55);
    }

    #[test]
    fn test_looper_snapshot_progress() {
        let mut looper = LooperEngine::new(44100.0);
        looper.set_bars(4);
        looper.play();

        let snap = looper.get_snapshot();
        assert_eq!(snap.current_bar, 1);
        assert_eq!(snap.current_beat, 1);
        assert_eq!(snap.progress, 0.0);
        assert!(snap.is_running);
    }

    #[test]
    fn test_metronome_not_in_recorded_loop() {
        let mut looper = LooperEngine::new(44100.0);
        looper.set_bpm(120.0);
        looper.set_bars(1);
        looper.metronome.set_enabled(true);
        looper.metronome.set_volume(1.0);

        // Arm track 0 for recording
        looper.record_track(0);

        let loop_len = looper.loop_length_samples;
        // Provide pure silence as user performance input
        let mut left = vec![0.0f32; loop_len];
        let mut right = vec![0.0f32; loop_len];

        // Process audio for 1 full loop pass with metronome enabled
        looper.process_stereo(&mut left, &mut right);

        // 1. Output monitor buffers MUST have received metronome clicks so the user heard the beat
        let max_output_l = left.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
        let max_output_r = right.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
        assert!(max_output_l > 0.1, "Output monitor must contain audible metronome clicks (got {max_output_l})");
        assert!(max_output_r > 0.1, "Output monitor must contain audible metronome clicks (got {max_output_r})");

        // 2. Track 0 recorded buffer MUST NOT contain any metronome click (must be pure 0.0)!
        let max_recorded_l = looper.tracks[0].buffer_l.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
        let max_recorded_r = looper.tracks[0].buffer_r.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
        assert_eq!(max_recorded_l, 0.0, "Recorded track left buffer must NOT contain any metronome sound");
        assert_eq!(max_recorded_r, 0.0, "Recorded track right buffer must NOT contain any metronome sound");

        // 3. Combined mix buffers for WAV export MUST NOT contain any metronome click!
        let (mix_l, mix_r) = looper.get_mix_buffers();
        let max_mix_l = mix_l.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
        let max_mix_r = mix_r.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
        assert_eq!(max_mix_l, 0.0, "WAV export mix buffers must NOT contain any metronome sound");
        assert_eq!(max_mix_r, 0.0, "WAV export mix buffers must NOT contain any metronome sound");
    }

    #[test]
    fn test_looper_note_recording_editing_and_quantization() {
        let mut looper = LooperEngine::new(44100.0);
        looper.set_bpm(120.0);
        looper.set_bars(2); // 32 steps

        // Arm track 0 for recording
        looper.record_track(0);
        assert_eq!(looper.tracks[0].state, TrackState::Recording);

        // Record a kick on step 0
        looper.playhead = 0;
        looper.record_note_on(0, 36, 110);
        looper.record_note_off(0, 36);

        // Advance playhead to slightly off-beat position (step 4.35)
        let samples_per_step = looper.loop_length_samples as f32 / 32.0;
        looper.playhead = (4.35 * samples_per_step) as usize;
        looper.record_note_on(0, 38, 100); // Snare
        looper.playhead = (5.35 * samples_per_step) as usize;
        looper.record_note_off(0, 38);

        // Verify notes recorded
        assert_eq!(looper.tracks[0].notes.len(), 2);
        let kick = &looper.tracks[0].notes[0];
        assert_eq!(kick.note, 36);
        assert_eq!(kick.off_beat, false);

        let snare = &looper.tracks[0].notes[1];
        assert_eq!(snare.note, 38);
        assert_eq!(snare.off_beat, true); // 4.35 is off-beat (> 0.08 diff)

        // Test dragging / updating note position (user aligns snare to step 4.0)
        let snare_id = snare.id.clone();
        looper.update_note(0, &snare_id, 4.0, 38);
        let updated_snare = &looper.tracks[0].notes[1];
        assert_eq!(updated_snare.start_step, 4.0);
        assert_eq!(updated_snare.off_beat, false);

        // Intentionally move snare to off-beat again (step 4.4)
        looper.update_note(0, &snare_id, 4.4, 38);
        assert_eq!(looper.tracks[0].notes[1].off_beat, true);

        // Test quantize track to 1/16th grid (1.0 step)
        looper.quantize_track(0, 1.0);
        assert_eq!(looper.tracks[0].notes[1].start_step, 4.0);
        assert_eq!(looper.tracks[0].notes[1].off_beat, false);

        // Test snapshot contains notes and peaks
        let snap = looper.get_snapshot();
        assert_eq!(snap.track1_notes.len(), 2);
    }

    #[test]
    fn test_piano_never_ends_up_on_drum_track() {
        let mut looper = LooperEngine::new(44100.0);
        looper.set_bpm(120.0);
        looper.set_bars(1);

        let frames = looper.loop_length_samples;
        let drums_silence = vec![0.0f32; frames];
        let piano_loud = vec![0.85f32; frames];
        let mut out_l = vec![0.0f32; frames];
        let mut out_r = vec![0.0f32; frames];

        // 1. Arm Track 0 (Drums) for recording
        looper.record_track(0);
        assert_eq!(looper.tracks[0].state, TrackState::Recording);

        // While Track 0 (Drums) is recording, user plays piano simultaneously (loud piano, silent drums)
        looper.process_isolated(
            &drums_silence,
            &drums_silence,
            &piano_loud,
            &piano_loud,
            &mut out_l,
            &mut out_r,
        );

        // Track 0 (Drums) buffer MUST remain completely 0.0 (piano audio NEVER bleeds onto drum track)
        let max_t0_l = looper.tracks[0].buffer_l.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
        let max_t0_r = looper.tracks[0].buffer_r.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
        assert_eq!(max_t0_l, 0.0, "Track 0 (Drums) must NOT contain any piano audio");
        assert_eq!(max_t0_r, 0.0, "Track 0 (Drums) must NOT contain any piano audio");

        // 2. Note event isolation:
        // While Track 0 is recording drums, playing piano notes MUST NEVER be recorded into Track 0
        looper.record_note_on(1, 60, 100);
        looper.record_note_off(1, 60);
        assert_eq!(looper.tracks[0].notes.len(), 0, "Piano note must never end up in Drum track");

        // Now arm Track 1 (Piano / Keys) as well
        looper.record_track(1);
        assert_eq!(looper.tracks[1].state, TrackState::Recording);

        // Record a piano note on Track 1
        looper.record_note_on(1, 60, 100);
        looper.record_note_off(1, 60);

        // Piano note MUST be strictly in Track 1, NEVER in Track 0 (Drums)
        assert_eq!(looper.tracks[0].notes.len(), 0, "Piano note must never end up in Drum track");
        assert_eq!(looper.tracks[1].notes.len(), 1, "Piano note must be in Keys track");
        assert_eq!(looper.tracks[1].notes[0].note, 60);

        // 3. Drum note event isolation: overdub drum pad on Track 0 while Track 1 is playing
        looper.overdub_track(0);
        looper.record_note_on(0, 36, 110);
        looper.record_note_off(0, 36);

        // Drum note MUST be strictly in Track 0, NEVER in Track 1
        assert_eq!(looper.tracks[0].notes.len(), 1, "Drum note must be in Drum track");
        assert_eq!(looper.tracks[0].notes[0].note, 36);
        assert_eq!(looper.tracks[1].notes.len(), 1, "Keys track notes must remain untouched");
    }
}
