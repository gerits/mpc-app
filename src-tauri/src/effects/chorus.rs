/// Stereo Modulated Chorus & Spatial Delay Effect.
#[derive(Debug, Clone)]
pub struct StereoChorus {
    sample_rate: f32,
    buffer_l: Vec<f32>,
    buffer_r: Vec<f32>,
    write_idx: usize,

    // LFO phase for modulation
    lfo_phase_l: f32,
    lfo_phase_r: f32,
    lfo_inc: f32,

    rate_hz: f32,
    depth_samples: f32,
    base_delay_samples: f32,
    feedback: f32,
    mix: f32,
}

impl StereoChorus {
    const MAX_DELAY_SAMPLES: usize = 4096;

    pub fn new(sample_rate: f32) -> Self {
        let mut chorus = Self {
            sample_rate: sample_rate.max(1000.0),
            buffer_l: vec![0.0; Self::MAX_DELAY_SAMPLES],
            buffer_r: vec![0.0; Self::MAX_DELAY_SAMPLES],
            write_idx: 0,
            lfo_phase_l: 0.0,
            lfo_phase_r: std::f32::consts::FRAC_PI_2, // 90 degree quadrature phase for stereo spread
            lfo_inc: 0.0,
            rate_hz: 1.0,
            depth_samples: 0.0,
            base_delay_samples: 0.0,
            feedback: 0.2,
            mix: 0.0,
        };
        chorus.update_parameters();
        chorus
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate.max(1000.0);
        self.update_parameters();
    }

    pub fn set_mix(&mut self, mix: f32) {
        self.mix = mix.clamp(0.0, 1.0);
    }

    pub fn set_rate(&mut self, rate_hz: f32) {
        self.rate_hz = rate_hz.clamp(0.1, 5.0);
        self.lfo_inc = 2.0 * std::f32::consts::PI * self.rate_hz / self.sample_rate;
    }

    pub fn set_depth(&mut self, depth_ms: f32) {
        self.depth_samples = (depth_ms.clamp(0.5, 10.0) * 0.001 * self.sample_rate).min(Self::MAX_DELAY_SAMPLES as f32 * 0.4);
    }

    fn update_parameters(&mut self) {
        self.lfo_inc = 2.0 * std::f32::consts::PI * self.rate_hz / self.sample_rate;
        // Nominal delay ~15 ms
        self.base_delay_samples = (15.0 * 0.001 * self.sample_rate).min(Self::MAX_DELAY_SAMPLES as f32 * 0.5);
        // Default modulation depth ~3 ms
        self.depth_samples = (3.0 * 0.001 * self.sample_rate).min(Self::MAX_DELAY_SAMPLES as f32 * 0.25);
    }

    pub fn clear(&mut self) {
        self.buffer_l.fill(0.0);
        self.buffer_r.fill(0.0);
        self.write_idx = 0;
    }

    #[inline]
    fn read_fractional(buffer: &[f32], read_pos: f32) -> f32 {
        let len = buffer.len() as f32;
        let pos = (read_pos % len + len) % len;
        let idx0 = pos.floor() as usize;
        let idx1 = if idx0 + 1 >= buffer.len() { 0 } else { idx0 + 1 };
        let frac = pos - pos.floor();

        // Linear interpolation
        buffer[idx0] * (1.0 - frac) + buffer[idx1] * frac
    }

    /// Process a single stereo frame through the chorus effect.
    #[inline]
    pub fn process_sample(&mut self, in_l: f32, in_r: f32) -> (f32, f32) {
        if self.mix <= 0.001 {
            return (in_l, in_r);
        }

        // Modulate delay times with quadrature LFOs
        let mod_l = self.lfo_phase_l.sin() * self.depth_samples;
        let mod_r = self.lfo_phase_r.sin() * self.depth_samples;

        // Advance LFO phases
        self.lfo_phase_l += self.lfo_inc;
        if self.lfo_phase_l > 2.0 * std::f32::consts::PI {
            self.lfo_phase_l -= 2.0 * std::f32::consts::PI;
        }
        self.lfo_phase_r += self.lfo_inc;
        if self.lfo_phase_r > 2.0 * std::f32::consts::PI {
            self.lfo_phase_r -= 2.0 * std::f32::consts::PI;
        }

        let delay_l = self.base_delay_samples + mod_l;
        let delay_r = self.base_delay_samples + mod_r;

        let read_pos_l = self.write_idx as f32 - delay_l;
        let read_pos_r = self.write_idx as f32 - delay_r;

        let wet_l = Self::read_fractional(&self.buffer_l, read_pos_l);
        let wet_r = Self::read_fractional(&self.buffer_r, read_pos_r);

        // Write input + feedback into delay lines
        self.buffer_l[self.write_idx] = in_l + wet_l * self.feedback;
        self.buffer_r[self.write_idx] = in_r + wet_r * self.feedback;

        self.write_idx += 1;
        if self.write_idx >= Self::MAX_DELAY_SAMPLES {
            self.write_idx = 0;
        }

        let dry = 1.0 - self.mix * 0.4;
        let wet_gain = self.mix * 0.8;

        (in_l * dry + wet_l * wet_gain, in_r * dry + wet_r * wet_gain)
    }

    /// Process stereo buffer in place.
    pub fn process_stereo(&mut self, left: &mut [f32], right: &mut [f32]) {
        assert_eq!(left.len(), right.len());
        if self.mix <= 0.001 {
            return;
        }

        for i in 0..left.len() {
            let (l, r) = self.process_sample(left[i], right[i]);
            left[i] = l;
            right[i] = r;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chorus_bypass_and_processing() {
        let mut chorus = StereoChorus::new(44100.0);
        chorus.set_mix(0.0);

        let mut left = vec![0.5; 128];
        let mut right = vec![0.5; 128];
        chorus.process_stereo(&mut left, &mut right);
        assert_eq!(left[0], 0.5);

        // Turn mix on
        chorus.set_mix(0.5);
        let mut input_l = vec![1.0; 256];
        let mut input_r = vec![1.0; 256];
        chorus.process_stereo(&mut input_l, &mut input_r);
        assert!(input_l[0] > 0.0);
    }
}
