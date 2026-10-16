/// 2-Pole Resonant State Variable Filter (Cytomic / Andy Simper topology).
/// Stable, click-free under fast modulation, and numerically well-behaved from 20 Hz to Nyquist.
#[derive(Debug, Clone)]
pub struct StateVariableFilter {
    sample_rate: f32,
    cutoff_hz: f32,
    q: f32,

    // Filter coefficients
    a1: f32,
    a2: f32,
    a3: f32,

    // Left channel internal states
    ic1_l: f32,
    ic2_l: f32,

    // Right channel internal states
    ic1_r: f32,
    ic2_r: f32,
}

impl StateVariableFilter {
    pub fn new(sample_rate: f32) -> Self {
        let mut filter = Self {
            sample_rate: sample_rate.max(1000.0),
            cutoff_hz: 20000.0,
            q: 0.707,
            a1: 0.0,
            a2: 0.0,
            a3: 0.0,
            ic1_l: 0.0,
            ic2_l: 0.0,
            ic1_r: 0.0,
            ic2_r: 0.0,
        };
        filter.update_coefficients();
        filter
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate.max(1000.0);
        self.update_coefficients();
    }

    pub fn set_parameters(&mut self, cutoff_hz: f32, q: f32) {
        // Clamp cutoff to valid audible range (up to 0.49 * sample_rate to prevent tangent blowup)
        let max_cutoff = self.sample_rate * 0.49;
        self.cutoff_hz = cutoff_hz.clamp(20.0, max_cutoff);
        self.q = q.clamp(0.5, 10.0);
        self.update_coefficients();
    }

    fn update_coefficients(&mut self) {
        let normalized = (std::f32::consts::PI * (self.cutoff_hz / self.sample_rate)).tan();
        let k = 1.0 / self.q;
        self.a1 = 1.0 / (1.0 + normalized * (normalized + k));
        self.a2 = normalized * self.a1;
        self.a3 = normalized * self.a2;
    }

    pub fn reset(&mut self) {
        self.ic1_l = 0.0;
        self.ic2_l = 0.0;
        self.ic1_r = 0.0;
        self.ic2_r = 0.0;
    }

    /// Process a single stereo sample frame through the resonant lowpass filter.
    #[inline]
    pub fn process_sample(&mut self, in_l: f32, in_r: f32) -> (f32, f32) {
        // Left channel
        let v3_l = in_l - self.ic2_l;
        let v1_l = self.a1 * self.ic1_l + self.a2 * v3_l;
        let v2_l = self.ic2_l + self.a2 * self.ic1_l + self.a3 * v3_l;
        self.ic1_l = 2.0 * v1_l - self.ic1_l;
        self.ic2_l = 2.0 * v2_l - self.ic2_l;

        // Right channel
        let v3_r = in_r - self.ic2_r;
        let v1_r = self.a1 * self.ic1_r + self.a2 * v3_r;
        let v2_r = self.ic2_r + self.a2 * self.ic1_r + self.a3 * v3_r;
        self.ic1_r = 2.0 * v1_r - self.ic1_r;
        self.ic2_r = 2.0 * v2_r - self.ic2_r;

        (v2_l, v2_r)
    }

    /// Process blocks of stereo samples in place.
    pub fn process_stereo(&mut self, left: &mut [f32], right: &mut [f32]) {
        assert_eq!(left.len(), right.len());
        // If filter is fully open (> 19.5kHz) and flat Q, bypass processing for CPU efficiency
        if self.cutoff_hz >= 19500.0 && self.q <= 0.72 {
            return;
        }

        for i in 0..left.len() {
            let (out_l, out_r) = self.process_sample(left[i], right[i]);
            left[i] = out_l;
            right[i] = out_r;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_attenuates_high_frequencies() {
        let mut filter = StateVariableFilter::new(44100.0);
        // Set low cutoff at 200 Hz
        filter.set_parameters(200.0, 1.0);

        // Generate 5000 Hz high frequency sine wave
        let len = 1000;
        let mut left = Vec::with_capacity(len);
        let mut right = Vec::with_capacity(len);
        for t in 0..len {
            let s = (t as f32 * 5000.0 * 2.0 * std::f32::consts::PI / 44100.0).sin();
            left.push(s);
            right.push(s);
        }

        filter.process_stereo(&mut left, &mut right);

        // Check steady state attenuation in the tail
        let tail_peak = left[500..].iter().map(|s| s.abs()).fold(0.0f32, f32::max);
        assert!(tail_peak < 0.1, "5000 Hz must be heavily attenuated by 200 Hz lowpass filter, got peak: {tail_peak}");
    }

    #[test]
    fn test_filter_passes_low_frequencies() {
        let mut filter = StateVariableFilter::new(44100.0);
        filter.set_parameters(1000.0, 0.707);

        // Generate 100 Hz low frequency sine wave
        let len = 1000;
        let mut left = Vec::with_capacity(len);
        let mut right = Vec::with_capacity(len);
        for t in 0..len {
            let s = (t as f32 * 100.0 * 2.0 * std::f32::consts::PI / 44100.0).sin();
            left.push(s);
            right.push(s);
        }

        filter.process_stereo(&mut left, &mut right);

        let tail_peak = left[500..].iter().map(|s| s.abs()).fold(0.0f32, f32::max);
        assert!(tail_peak > 0.85, "100 Hz must pass through 1000 Hz lowpass filter unattenuated, got peak: {tail_peak}");
    }
}
