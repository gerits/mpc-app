/// Comb filter with lowpass feedback used in Schroeder/Freeverb reverberator.
#[derive(Debug, Clone)]
struct CombFilter {
    buffer: Vec<f32>,
    buffer_idx: usize,
    feedback: f32,
    filter_store: f32,
    damp1: f32,
    damp2: f32,
}

impl CombFilter {
    fn new(size: usize) -> Self {
        Self {
            buffer: vec![0.0; size.max(1)],
            buffer_idx: 0,
            feedback: 0.5,
            filter_store: 0.0,
            damp1: 0.5,
            damp2: 0.5,
        }
    }

    fn set_damp(&mut self, damp: f32) {
        self.damp1 = damp;
        self.damp2 = 1.0 - damp;
    }

    fn set_feedback(&mut self, feedback: f32) {
        self.feedback = feedback;
    }

    #[inline]
    fn process(&mut self, input: f32) -> f32 {
        let output = self.buffer[self.buffer_idx];
        self.filter_store = output * self.damp2 + self.filter_store * self.damp1;
        self.buffer[self.buffer_idx] = input + self.filter_store * self.feedback;

        self.buffer_idx += 1;
        if self.buffer_idx >= self.buffer.len() {
            self.buffer_idx = 0;
        }

        output
    }

    fn clear(&mut self) {
        self.buffer.fill(0.0);
        self.filter_store = 0.0;
        self.buffer_idx = 0;
    }
}

/// Allpass filter with feedback for reverberant diffusion.
#[derive(Debug, Clone)]
struct AllpassFilter {
    buffer: Vec<f32>,
    buffer_idx: usize,
    feedback: f32,
}

impl AllpassFilter {
    fn new(size: usize) -> Self {
        Self {
            buffer: vec![0.0; size.max(1)],
            buffer_idx: 0,
            feedback: 0.5,
        }
    }

    #[inline]
    fn process(&mut self, input: f32) -> f32 {
        let buf_out = self.buffer[self.buffer_idx];
        let output = -input + buf_out;
        self.buffer[self.buffer_idx] = input + buf_out * self.feedback;

        self.buffer_idx += 1;
        if self.buffer_idx >= self.buffer.len() {
            self.buffer_idx = 0;
        }

        output
    }

    fn clear(&mut self) {
        self.buffer.fill(0.0);
        self.buffer_idx = 0;
    }
}

const COMB_TUNING_L: [usize; 8] = [1116, 1188, 1277, 1356, 1422, 1491, 1557, 1617];
const ALLPASS_TUNING_L: [usize; 4] = [556, 441, 341, 225];
const STEREO_SPREAD: usize = 23;

/// Freeverb-style stereo reverberator providing rich ambience with low CPU footprint.
#[derive(Debug, Clone)]
pub struct StereoReverb {
    sample_rate: f32,
    combs_l: Vec<CombFilter>,
    combs_r: Vec<CombFilter>,
    allpasses_l: Vec<AllpassFilter>,
    allpasses_r: Vec<AllpassFilter>,

    room_size: f32,
    damp: f32,
    wet: f32,
    dry: f32,
    width: f32,
}

impl StereoReverb {
    pub fn new(sample_rate: f32) -> Self {
        let mut rev = Self {
            sample_rate: sample_rate.max(1000.0),
            combs_l: Vec::new(),
            combs_r: Vec::new(),
            allpasses_l: Vec::new(),
            allpasses_r: Vec::new(),
            room_size: 0.7,
            damp: 0.2,
            wet: 0.0,
            dry: 1.0,
            width: 1.0,
        };
        rev.init_buffers();
        rev
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate.max(1000.0);
        self.init_buffers();
    }

    fn init_buffers(&mut self) {
        let scale = self.sample_rate / 44100.0;
        self.combs_l.clear();
        self.combs_r.clear();
        self.allpasses_l.clear();
        self.allpasses_r.clear();

        for &tuning in &COMB_TUNING_L {
            let size_l = (tuning as f32 * scale).round() as usize;
            let size_r = ((tuning + STEREO_SPREAD) as f32 * scale).round() as usize;
            self.combs_l.push(CombFilter::new(size_l));
            self.combs_r.push(CombFilter::new(size_r));
        }

        for &tuning in &ALLPASS_TUNING_L {
            let size_l = (tuning as f32 * scale).round() as usize;
            let size_r = ((tuning + STEREO_SPREAD) as f32 * scale).round() as usize;
            self.allpasses_l.push(AllpassFilter::new(size_l));
            self.allpasses_r.push(AllpassFilter::new(size_r));
        }

        self.update_parameters();
    }

    pub fn set_wet(&mut self, wet: f32) {
        self.wet = wet.clamp(0.0, 1.0);
        // Constant acoustic energy crossfade: dry slightly recedes as wet increases
        self.dry = (1.0 - self.wet * 0.3).clamp(0.0, 1.0);
    }

    pub fn set_room_size(&mut self, room_size: f32) {
        self.room_size = room_size.clamp(0.0, 1.0);
        self.update_parameters();
    }

    pub fn set_damping(&mut self, damp: f32) {
        self.damp = damp.clamp(0.0, 1.0);
        self.update_parameters();
    }

    fn update_parameters(&mut self) {
        // Feedback maps 0.7 .. 0.97
        let feedback = 0.7 + self.room_size * 0.27;
        let damp_val = self.damp * 0.4;

        for comb in self.combs_l.iter_mut().chain(self.combs_r.iter_mut()) {
            comb.set_feedback(feedback);
            comb.set_damp(damp_val);
        }
    }

    pub fn clear(&mut self) {
        for comb in self.combs_l.iter_mut().chain(self.combs_r.iter_mut()) {
            comb.clear();
        }
        for ap in self.allpasses_l.iter_mut().chain(self.allpasses_r.iter_mut()) {
            ap.clear();
        }
    }

    /// Process a single stereo frame through the reverb.
    #[inline]
    pub fn process_sample(&mut self, in_l: f32, in_r: f32) -> (f32, f32) {
        if self.wet <= 0.001 {
            return (in_l, in_r);
        }

        let input = (in_l + in_r) * 0.015;

        // Parallel Comb filters
        let mut out_l = 0.0;
        let mut out_r = 0.0;
        for comb in &mut self.combs_l {
            out_l += comb.process(input);
        }
        for comb in &mut self.combs_r {
            out_r += comb.process(input);
        }

        // Series Allpass filters
        for ap in &mut self.allpasses_l {
            out_l = ap.process(out_l);
        }
        for ap in &mut self.allpasses_r {
            out_r = ap.process(out_r);
        }

        let wet_1 = self.wet * (self.width * 0.5 + 0.5);
        let wet_2 = self.wet * ((1.0 - self.width) * 0.5);

        let final_l = in_l * self.dry + out_l * wet_1 + out_r * wet_2;
        let final_r = in_r * self.dry + out_r * wet_1 + out_l * wet_2;

        (final_l, final_r)
    }

    /// Process stereo buffer in place.
    pub fn process_stereo(&mut self, left: &mut [f32], right: &mut [f32]) {
        assert_eq!(left.len(), right.len());
        if self.wet <= 0.001 {
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
    fn test_reverb_wet_dry_mix() {
        let mut reverb = StereoReverb::new(44100.0);
        reverb.set_wet(0.0);

        let mut left = vec![1.0; 256];
        let mut right = vec![1.0; 256];
        reverb.process_stereo(&mut left, &mut right);
        // With wet 0.0, dry is 1.0
        assert_eq!(left[0], 1.0);
        assert_eq!(right[0], 1.0);

        // Turn wet up to 0.5
        reverb.set_wet(0.5);
        let mut left_impulse = vec![0.0; 2048];
        let mut right_impulse = vec![0.0; 2048];
        left_impulse[0] = 1.0;
        right_impulse[0] = 1.0;
        reverb.process_stereo(&mut left_impulse, &mut right_impulse);

        // Reverb tail must be non-zero after the comb delay line
        let tail_sum: f32 = left_impulse[1..].iter().map(|s| s.abs()).sum();
        assert!(tail_sum > 0.001, "Reverb must produce diffuse tail");
    }
}
