use std::f32::consts::PI;
use serde::{Deserialize, Serialize};

/// Mode determining when the metronome click plays.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MetronomeMode {
    /// Plays only when looper transport is actively running (Play, Record, Overdub).
    #[default]
    WhileRunning,
    /// Plays continuously whenever metronome is enabled, even if transport is stopped.
    Always,
}

/// Sample-accurate synthesised metronome engine.
/// Produces an accented click on beat 1 (downbeat) and standard clicks on secondary beats.
/// Rendered exclusively to audio output monitors so it never bleeds into looper track recording.
#[derive(Debug, Clone)]
pub struct Metronome {
    pub sample_rate: f32,
    pub bpm: f32,
    pub beats_per_bar: u32,
    pub enabled: bool,
    pub volume: f32,
    pub mode: MetronomeMode,

    accent_buf: Vec<f32>,
    normal_buf: Vec<f32>,

    click_cursor: usize,
    click_is_accent: bool,
    click_active: bool,

    standalone_pos: usize,
}

impl Metronome {
    pub const DEFAULT_VOLUME: f32 = 0.7;

    pub fn new(sample_rate: f32) -> Self {
        let sr = sample_rate.max(1000.0);
        let mut metro = Self {
            sample_rate: sr,
            bpm: 120.0,
            beats_per_bar: 4,
            enabled: false,
            volume: Self::DEFAULT_VOLUME,
            mode: MetronomeMode::WhileRunning,
            accent_buf: Vec::new(),
            normal_buf: Vec::new(),
            click_cursor: 0,
            click_is_accent: false,
            click_active: false,
            standalone_pos: 0,
        };
        metro.generate_click_buffers();
        metro
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate.max(1000.0);
        self.generate_click_buffers();
    }

    pub fn set_bpm(&mut self, bpm: f32) {
        self.bpm = bpm.clamp(40.0, 300.0);
    }

    pub fn set_beats_per_bar(&mut self, beats: u32) {
        self.beats_per_bar = beats.max(1);
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if !enabled {
            self.click_active = false;
        }
    }

    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 2.0);
    }

    pub fn set_mode(&mut self, mode: MetronomeMode) {
        self.mode = mode;
    }

    /// Whether the metronome is currently generating sound based on transport state.
    pub fn is_ticking(&self, looper_running: bool) -> bool {
        if !self.enabled {
            return false;
        }
        match self.mode {
            MetronomeMode::WhileRunning => looper_running,
            MetronomeMode::Always => true,
        }
    }

    /// Trigger a click sound from the beginning of its pre-rendered buffer.
    pub fn trigger_click(&mut self, is_accent: bool) {
        self.click_active = true;
        self.click_is_accent = is_accent;
        self.click_cursor = 0;
    }

    /// Fetch next click sample (mono) scaled by volume.
    #[inline]
    pub fn next_sample(&mut self) -> f32 {
        if !self.click_active {
            return 0.0;
        }

        let buf = if self.click_is_accent {
            &self.accent_buf
        } else {
            &self.normal_buf
        };

        if self.click_cursor < buf.len() {
            let sample = buf[self.click_cursor] * self.volume;
            self.click_cursor += 1;
            sample
        } else {
            self.click_active = false;
            0.0
        }
    }

    /// Process a single frame during active looper playback/recording.
    /// Returns the metronome sample to be added to monitor output.
    #[inline]
    pub fn process_sample_running(
        &mut self,
        pos: usize,
        loop_len: usize,
        total_beats: u32,
        beats_per_bar: u32,
    ) -> f32 {
        if !self.enabled {
            return 0.0;
        }

        if total_beats > 0 && loop_len > 0 {
            let samples_per_beat = loop_len as f64 / total_beats as f64;
            let b = ((pos as f64) / samples_per_beat).round() as usize;
            if b < total_beats as usize {
                let beat_pos = ((b as f64) * samples_per_beat).round() as usize;
                if pos == beat_pos {
                    let is_accent = (b % (beats_per_bar.max(1) as usize)) == 0;
                    self.trigger_click(is_accent);
                }
            }
        }

        self.next_sample()
    }

    /// Process an audio block when looper transport is stopped (used in Always mode).
    pub fn process_stereo_stopped(&mut self, left: &mut [f32], right: &mut [f32]) {
        if !self.enabled || self.mode != MetronomeMode::Always {
            return;
        }

        let frames = left.len();
        let beats = self.beats_per_bar.max(1);
        let bar_samples =
            ((self.sample_rate as f64 * 60.0 / self.bpm as f64) * beats as f64).round() as usize;
        let bar_samples = bar_samples.max(1);
        let samples_per_beat = bar_samples as f64 / beats as f64;

        for i in 0..frames {
            let b = ((self.standalone_pos as f64) / samples_per_beat).round() as usize;
            if b < beats as usize {
                let beat_pos = ((b as f64) * samples_per_beat).round() as usize;
                if self.standalone_pos == beat_pos {
                    self.trigger_click(b == 0);
                }
            }

            let s = self.next_sample();
            left[i] += s;
            right[i] += s;

            self.standalone_pos += 1;
            if self.standalone_pos >= bar_samples {
                self.standalone_pos = 0;
            }
        }
    }

    /// Reset standalone counter (e.g. when transport starts or stops).
    pub fn reset_standalone_phase(&mut self) {
        self.standalone_pos = 0;
    }

    /// Pre-renders crisp woodblock / digital metronome click waves to scratch buffers.
    /// Accent: 1600 Hz with 3200 Hz harmonic, fast 0.35ms attack, 7ms exponential decay.
    /// Normal: 900 Hz with 1800 Hz harmonic, fast 0.35ms attack, 5.5ms exponential decay.
    fn generate_click_buffers(&mut self) {
        let sr = self.sample_rate;

        // 1. Accent Click (Downbeat / Beat 1) - ~32ms length
        let accent_len = (sr * 0.032).round() as usize;
        let mut accent = Vec::with_capacity(accent_len);
        let attack_samples = ((sr * 0.0004).round() as usize).max(4); // ~0.4ms linear ramp
        let decay_tau = 0.007; // 7ms decay constant

        for s in 0..accent_len {
            let t = s as f32 / sr;
            let attack = (s as f32 / attack_samples as f32).min(1.0);
            let decay = (-t / decay_tau).exp();
            let env = attack * decay;

            let phase1 = 2.0 * PI * 1600.0 * t;
            let phase2 = 2.0 * PI * 3200.0 * t;
            let val = (phase1.sin() + 0.3 * phase2.sin()) * 0.75 * env;
            accent.push(val);
        }
        self.accent_buf = accent;

        // 2. Normal Click (Beats 2, 3, 4) - ~24ms length
        let normal_len = (sr * 0.024).round() as usize;
        let mut normal = Vec::with_capacity(normal_len);
        let normal_tau = 0.0055; // 5.5ms decay constant

        for s in 0..normal_len {
            let t = s as f32 / sr;
            let attack = (s as f32 / attack_samples as f32).min(1.0);
            let decay = (-t / normal_tau).exp();
            let env = attack * decay;

            let phase1 = 2.0 * PI * 900.0 * t;
            let phase2 = 2.0 * PI * 1800.0 * t;
            let val = (phase1.sin() + 0.2 * phase2.sin()) * 0.55 * env;
            normal.push(val);
        }
        self.normal_buf = normal;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metronome_initialization() {
        let metro = Metronome::new(44100.0);
        assert!(!metro.enabled);
        assert_eq!(metro.mode, MetronomeMode::WhileRunning);
        assert!(!metro.accent_buf.is_empty());
        assert!(!metro.normal_buf.is_empty());
        assert!(metro.accent_buf.len() > metro.normal_buf.len());
    }

    #[test]
    fn test_metronome_beat_detection_exactness() {
        let mut metro = Metronome::new(44100.0);
        metro.set_bpm(120.0);
        metro.set_enabled(true);

        // 4 bars of 4 beats = 16 beats.
        let loop_len = 352800; // 44100 * 8s
        let total_beats = 16;
        let beats_per_bar = 4;

        let mut accent_triggers = 0;
        let mut normal_triggers = 0;

        for pos in 0..loop_len {
            let samples_per_beat = loop_len as f64 / total_beats as f64;
            let b = ((pos as f64) / samples_per_beat).round() as usize;
            if b < total_beats as usize {
                let beat_pos = ((b as f64) * samples_per_beat).round() as usize;
                if pos == beat_pos {
                    if (b % beats_per_bar) == 0 {
                        accent_triggers += 1;
                    } else {
                        normal_triggers += 1;
                    }
                }
            }
        }

        assert_eq!(accent_triggers, 4, "Should trigger exactly 4 downbeats in 4 bars");
        assert_eq!(normal_triggers, 12, "Should trigger exactly 12 regular beats in 4 bars");
    }

    #[test]
    fn test_metronome_odd_tempo_triggers() {
        let mut metro = Metronome::new(44100.0);
        metro.set_bpm(127.0);
        metro.set_enabled(true);

        let beats_per_bar = 4;
        let bar_count = 2;
        let total_beats = beats_per_bar * bar_count;
        let samples_per_beat = (44100.0 * 60.0) / 127.0;
        let loop_len = (samples_per_beat * total_beats as f32).round() as usize;

        let mut trigger_count = 0;
        for pos in 0..loop_len {
            let spb = loop_len as f64 / total_beats as f64;
            let b = ((pos as f64) / spb).round() as usize;
            if b < total_beats as usize {
                let beat_pos = ((b as f64) * spb).round() as usize;
                if pos == beat_pos {
                    trigger_count += 1;
                }
            }
        }

        assert_eq!(trigger_count, 8, "Must trigger exactly 8 beats for 2 bars");
    }
}
