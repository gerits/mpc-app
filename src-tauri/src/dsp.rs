use serde::{Deserialize, Serialize};

use crate::effects::{StateVariableFilter, StereoChorus, StereoReverb};

/// Values of the 8 macro knobs mapped directly from Akai MPK mini 3 knobs K1–K8 (CC 70–77).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MacroParams {
    /// K1: Filter Cutoff (0.0 ..= 1.0, mapped logarithmically to 20 Hz ..= 20,000 Hz)
    pub filter_cutoff: f32,
    /// K2: Filter Resonance (0.0 ..= 1.0, mapped to Q 0.707 ..= 8.0)
    pub filter_resonance: f32,
    /// K3: Attack Envelope Modifier (0.0 ..= 1.0, 0.0 = punchy/immediate, 1.0 = slow swell)
    pub attack_modifier: f32,
    /// K4: Release Envelope Modifier (0.0 ..= 1.0, scales release time tail)
    pub release_modifier: f32,
    /// K5: Stereo Reverb Mix (0.0 ..= 1.0 wet crossfade)
    pub reverb_mix: f32,
    /// K6: Stereo Chorus / Delay Mix (0.0 ..= 1.0)
    pub chorus_mix: f32,
    /// K7: Master Output Gain (0.0 ..= 1.0, 0.8 = 0 dB unity, 1.0 = +3 dB)
    pub master_gain: f32,
    /// K8: Keys vs Drums Balance (0.0 = drums only, 0.5 = 50/50 balance, 1.0 = keys only)
    pub balance: f32,
}

impl Default for MacroParams {
    fn default() -> Self {
        Self {
            filter_cutoff: 1.0,     // Fully open filter
            filter_resonance: 0.0,  // Flat Q (0.707)
            attack_modifier: 0.0,   // Punchy natural attack
            release_modifier: 0.5,  // Standard soundfont release
            reverb_mix: 0.15,       // Subtle acoustic room warmth
            chorus_mix: 0.0,        // Dry chorus by default
            master_gain: 0.8,       // 0 dB unity gain
            balance: 0.5,           // Equal 50/50 balance between keys and drums
        }
    }
}

/// Realtime DSP processing chain executing inside the low-latency audio callback.
/// Applies resonant SVF filtering, stereo chorus, Freeverb ambience, master gain,
/// and soft saturation with smooth zipper-noise-free parameter interpolation.
pub struct DspChain {
    sample_rate: f32,
    filter: StateVariableFilter,
    chorus: StereoChorus,
    reverb: StereoReverb,

    target_params: MacroParams,
    smoothed_params: MacroParams,
    smoothing_factor: f32,
}

impl DspChain {
    pub fn new(sample_rate: f32) -> Self {
        let sr = sample_rate.max(1000.0);
        let defaults = MacroParams::default();

        // 20ms one-pole smoothing time constant to eradicate parameter stepping/zipper noise
        let time_constant_sec = 0.020;
        let smoothing_factor = 1.0 - (-1.0 / (sr * time_constant_sec)).exp();

        let mut chain = Self {
            sample_rate: sr,
            filter: StateVariableFilter::new(sr),
            chorus: StereoChorus::new(sr),
            reverb: StereoReverb::new(sr),
            target_params: defaults,
            smoothed_params: defaults,
            smoothing_factor,
        };

        chain.update_dsp_targets();
        chain
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate.max(1000.0);
        self.filter.set_sample_rate(self.sample_rate);
        self.chorus.set_sample_rate(self.sample_rate);
        self.reverb.set_sample_rate(self.sample_rate);

        let time_constant_sec = 0.020;
        self.smoothing_factor = 1.0 - (-1.0 / (self.sample_rate * time_constant_sec)).exp();
    }

    /// Update a macro parameter from rotary knob index 0..=7 (CC 70..=77).
    pub fn set_knob(&mut self, knob_index: u8, normalized_value: f32) {
        let val = normalized_value.clamp(0.0, 1.0);
        match knob_index {
            0 => self.target_params.filter_cutoff = val,
            1 => self.target_params.filter_resonance = val,
            2 => self.target_params.attack_modifier = val,
            3 => self.target_params.release_modifier = val,
            4 => self.target_params.reverb_mix = val,
            5 => self.target_params.chorus_mix = val,
            6 => self.target_params.master_gain = val,
            7 => self.target_params.balance = val,
            _ => {}
        }
    }

    /// Read current target value for a knob 0..=7.
    pub fn get_knob(&self, knob_index: u8) -> f32 {
        match knob_index {
            0 => self.target_params.filter_cutoff,
            1 => self.target_params.filter_resonance,
            2 => self.target_params.attack_modifier,
            3 => self.target_params.release_modifier,
            4 => self.target_params.reverb_mix,
            5 => self.target_params.chorus_mix,
            6 => self.target_params.master_gain,
            7 => self.target_params.balance,
            _ => 0.0,
        }
    }

    pub fn target_params(&self) -> &MacroParams {
        &self.target_params
    }

    pub fn smoothed_params(&self) -> &MacroParams {
        &self.smoothed_params
    }

    /// Calculate keys vs drums gain multipliers from K8 balance parameter.
    /// - 0.0: Drums 100%, Keys 0%
    /// - 0.5: Keys 100%, Drums 100% (equal balance)
    /// - 1.0: Keys 100%, Drums 0%
    #[inline]
    pub fn get_keys_drums_gains(&self) -> (f32, f32) {
        let bal = self.smoothed_params.balance;
        let keys_gain = (bal * 2.0).min(1.0);
        let drums_gain = ((1.0 - bal) * 2.0).min(1.0);
        (keys_gain, drums_gain)
    }

    /// Convert normalized cutoff (0..1) to Hertz logarithmically (20 Hz .. 20,000 Hz).
    #[inline]
    fn cutoff_to_hz(normalized: f32) -> f32 {
        20.0 * (1000.0f32).powf(normalized.clamp(0.0, 1.0))
    }

    /// Convert normalized resonance (0..1) to Q factor (0.707 .. 8.0).
    #[inline]
    fn resonance_to_q(normalized: f32) -> f32 {
        0.707 + normalized.clamp(0.0, 1.0).powi(2) * 7.293
    }

    /// Convert normalized master gain (0..1) to linear multiplier.
    /// 0.0 -> 0.0 (muted)
    /// 0.8 -> 1.0 (unity 0 dB)
    /// 1.0 -> 1.412 (+3 dB headroom boost)
    #[inline]
    fn master_to_linear(normalized: f32) -> f32 {
        let n = normalized.clamp(0.0, 1.0);
        if n <= 0.8 {
            (n / 0.8).powi(2)
        } else {
            1.0 + (n - 0.8) / 0.2 * 0.412
        }
    }

    /// Update internal DSP components with the smoothed parameter values.
    fn update_dsp_targets(&mut self) {
        let cutoff = Self::cutoff_to_hz(self.smoothed_params.filter_cutoff);
        let q = Self::resonance_to_q(self.smoothed_params.filter_resonance);
        self.filter.set_parameters(cutoff, q);

        self.chorus.set_mix(self.smoothed_params.chorus_mix);
        self.reverb.set_wet(self.smoothed_params.reverb_mix);
    }

    /// Reset internal state buffers and clear tail memory.
    pub fn reset(&mut self) {
        self.filter.reset();
        self.chorus.clear();
        self.reverb.clear();
    }

    /// Process a stereo block through the complete DSP effect chain.
    pub fn process_stereo(&mut self, left: &mut [f32], right: &mut [f32]) {
        assert_eq!(left.len(), right.len());
        let frames = left.len();
        if frames == 0 {
            return;
        }

        // Interpolate smoothed parameters toward targets for this block
        let alpha = self.smoothing_factor;
        self.smoothed_params.filter_cutoff += alpha * (self.target_params.filter_cutoff - self.smoothed_params.filter_cutoff);
        self.smoothed_params.filter_resonance += alpha * (self.target_params.filter_resonance - self.smoothed_params.filter_resonance);
        self.smoothed_params.attack_modifier += alpha * (self.target_params.attack_modifier - self.smoothed_params.attack_modifier);
        self.smoothed_params.release_modifier += alpha * (self.target_params.release_modifier - self.smoothed_params.release_modifier);
        self.smoothed_params.reverb_mix += alpha * (self.target_params.reverb_mix - self.smoothed_params.reverb_mix);
        self.smoothed_params.chorus_mix += alpha * (self.target_params.chorus_mix - self.smoothed_params.chorus_mix);
        self.smoothed_params.master_gain += alpha * (self.target_params.master_gain - self.smoothed_params.master_gain);
        self.smoothed_params.balance += alpha * (self.target_params.balance - self.smoothed_params.balance);

        self.update_dsp_targets();

        // 1. Resonant SVF Lowpass Filter (K1, K2)
        self.filter.process_stereo(left, right);

        // 2. Stereo Chorus & Modulated Delay (K6)
        self.chorus.process_stereo(left, right);

        // 3. Stereo Freeverb Ambience (K5)
        self.reverb.process_stereo(left, right);

        // 4. Master Output Gain & Musical Soft Saturation Limiter (K7)
        let master_gain = Self::master_to_linear(self.smoothed_params.master_gain);

        for i in 0..frames {
            let l = left[i] * master_gain;
            let r = right[i] * master_gain;

            // Soft saturation transfer curve (tanh-like approximation)
            // Linear in -0.7 .. +0.7 range, smooth warm saturation above 0.7, hard ceiling at 1.0
            left[i] = Self::soft_saturate(l);
            right[i] = Self::soft_saturate(r);
        }
    }

    /// Fast soft saturation curve: transparent below 0.7, adds subtle analog harmonic warmth on peaks.
    #[inline]
    fn soft_saturate(x: f32) -> f32 {
        if x > 0.7 {
            let over = x - 0.7;
            0.7 + 0.28 * (over / (1.0 + over))
        } else if x < -0.7 {
            let under = -x - 0.7;
            -(0.7 + 0.28 * (under / (1.0 + under)))
        } else {
            x
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dsp_chain_initialization() {
        let chain = DspChain::new(44100.0);
        let p = chain.target_params();
        assert_eq!(p.filter_cutoff, 1.0);
        assert_eq!(p.balance, 0.5);

        let (kg, dg) = chain.get_keys_drums_gains();
        assert_eq!(kg, 1.0);
        assert_eq!(dg, 1.0);
    }

    #[test]
    fn test_knob_mapping() {
        let mut chain = DspChain::new(44100.0);
        chain.set_knob(0, 0.4); // Filter Cutoff
        chain.set_knob(1, 0.8); // Resonance
        chain.set_knob(4, 0.6); // Reverb Mix
        chain.set_knob(7, 0.0); // Drums only

        assert_eq!(chain.get_knob(0), 0.4);
        assert_eq!(chain.get_knob(1), 0.8);
        assert_eq!(chain.get_knob(4), 0.6);
        assert_eq!(chain.get_knob(7), 0.0);
    }

    #[test]
    fn test_balance_gains() {
        let mut chain = DspChain::new(44100.0);
        // Default 0.5 = 100% keys, 100% drums
        assert_eq!(chain.get_keys_drums_gains(), (1.0, 1.0));

        // Set to drums only (0.0) and force-step smoothing
        chain.set_knob(7, 0.0);
        chain.smoothed_params.balance = 0.0;
        let (kg, dg) = chain.get_keys_drums_gains();
        assert_eq!(kg, 0.0);
        assert_eq!(dg, 1.0);

        // Set to keys only (1.0)
        chain.set_knob(7, 1.0);
        chain.smoothed_params.balance = 1.0;
        let (kg, dg) = chain.get_keys_drums_gains();
        assert_eq!(kg, 1.0);
        assert_eq!(dg, 0.0);
    }

    #[test]
    fn test_soft_saturation() {
        // Linear range below 0.7
        assert_eq!(DspChain::soft_saturate(0.5), 0.5);
        assert_eq!(DspChain::soft_saturate(-0.5), -0.5);

        // Saturation above 0.7 remains <= 1.0
        let sat = DspChain::soft_saturate(2.0);
        assert!(sat < 1.0);
        assert!(sat > 0.7);

        let neg_sat = DspChain::soft_saturate(-2.0);
        assert!(neg_sat > -1.0);
        assert!(neg_sat < -0.7);
    }

    #[test]
    fn test_filter_sweep_no_glitches() {
        let mut chain = DspChain::new(44100.0);
        let mut left = vec![0.5; 256];
        let mut right = vec![0.5; 256];

        // Sweep filter rapidly from 1.0 down to 0.1
        for step in 0..10 {
            let cutoff = 1.0 - (step as f32 * 0.09);
            chain.set_knob(0, cutoff);
            chain.process_stereo(&mut left, &mut right);

            // Ensure output is finite and bounded
            for &sample in &left {
                assert!(sample.is_finite(), "Output must be finite");
                assert!(sample.abs() <= 1.0, "Output must be bounded");
            }
        }
    }
}
