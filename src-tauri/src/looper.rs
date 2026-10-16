use serde::{Deserialize, Serialize};

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

/// A single stereo loop track buffer.
#[derive(Debug, Clone)]
pub struct LooperTrack {
    pub state: TrackState,
    pub buffer_l: Vec<f32>,
    pub buffer_r: Vec<f32>,
    pub volume: f32,
    pub has_audio: bool,
}

impl LooperTrack {
    pub fn new(capacity: usize) -> Self {
        Self {
            state: TrackState::Empty,
            buffer_l: vec![0.0; capacity],
            buffer_r: vec![0.0; capacity],
            volume: 1.0,
            has_audio: false,
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
        }
    }

    pub fn calculate_loop_samples(sample_rate: f32, bpm: f32, beats_per_bar: u32, bar_count: u32) -> usize {
        let samples_per_beat = (sample_rate * 60.0) / bpm.clamp(40.0, 300.0);
        let samples_per_bar = samples_per_beat * (beats_per_bar.max(1) as f32);
        (samples_per_bar * (bar_count.max(1) as f32)).round() as usize
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate.max(1000.0);
        self.recalculate_length();
    }

    pub fn set_bpm(&mut self, bpm: f32) {
        self.bpm = bpm.clamp(40.0, 300.0);
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
                // Clear existing track audio for fresh recording pass
                self.tracks[track_idx].buffer_l.fill(0.0);
                self.tracks[track_idx].buffer_r.fill(0.0);
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
    }

    /// Stop transport playback and rewind playhead to beginning.
    pub fn stop(&mut self) {
        self.is_running = false;
        self.playhead = 0;

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

    /// Realtime processing callback inside the audio engine thread.
    /// Ingests live performance audio when recording/overdubbing and mixes loop audio into output.
    pub fn process_stereo(&mut self, left: &mut [f32], right: &mut [f32]) {
        assert_eq!(left.len(), right.len());
        if !self.is_running || self.loop_length_samples == 0 {
            return;
        }

        let frames = left.len();
        let loop_len = self.loop_length_samples;
        let decay = self.overdub_decay;

        for i in 0..frames {
            let pos = self.playhead;
            let in_l = left[i];
            let in_r = right[i];

            // 1. Process recording & playback across both tracks
            for track in &mut self.tracks {
                match track.state {
                    TrackState::Recording => {
                        // Capture live audio into buffer
                        track.buffer_l[pos] = in_l;
                        track.buffer_r[pos] = in_r;
                    }
                    TrackState::Overdubbing => {
                        let existing_l = track.buffer_l[pos];
                        let existing_r = track.buffer_r[pos];
                        // Layer incoming audio on top of existing loop
                        track.buffer_l[pos] = existing_l * decay + in_l;
                        track.buffer_r[pos] = existing_r * decay + in_r;

                        // Add existing sound into current monitor output
                        left[i] += existing_l * track.volume;
                        right[i] += existing_r * track.volume;
                    }
                    TrackState::Playing => {
                        // Mix loop buffer into output
                        left[i] += track.buffer_l[pos] * track.volume;
                        right[i] += track.buffer_r[pos] * track.volume;
                    }
                    TrackState::Muted | TrackState::Empty => {}
                }
            }

            // 2. Advance playhead
            self.playhead += 1;
            if self.playhead >= loop_len {
                self.playhead = 0;

                // Seamless loop transition: any track finishing its first record pass loops automatically
                for track in &mut self.tracks {
                    if track.state == TrackState::Recording {
                        track.state = TrackState::Playing;
                        // Apply micro crossfade across 32 samples at the loop seam
                        Self::apply_seam_smoothing(&mut track.buffer_l, &mut track.buffer_r, loop_len);
                    }
                }
            }
        }
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
}
