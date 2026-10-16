use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use hound::{SampleFormat, WavSpec, WavWriter};

use crate::looper::LooperEngine;

/// Export raw stereo floating-point audio buffers to a broadcast-standard 24-bit PCM stereo WAV file.
pub fn export_to_wav<P: AsRef<Path>>(
    left: &[f32],
    right: &[f32],
    sample_rate: u32,
    path: P,
) -> Result<(), String> {
    if left.len() != right.len() {
        return Err("Left and right channels must have equal sample lengths".to_string());
    }
    if left.is_empty() {
        return Err("Cannot export empty audio buffer".to_string());
    }

    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create export folder: {e}"))?;
        }
    }

    let spec = WavSpec {
        channels: 2,
        sample_rate,
        bits_per_sample: 24,
        sample_format: SampleFormat::Int,
    };

    let mut writer = WavWriter::create(path, spec)
        .map_err(|e| format!("Failed to initialize WAV writer at {}: {e}", path.display()))?;

    // 24-bit signed integer full scale range: -8,388,608 ..= 8,388,607
    let amplitude = 8_388_607.0f32;

    for i in 0..left.len() {
        let l_sample = (left[i].clamp(-1.0, 1.0) * amplitude).round() as i32;
        let r_sample = (right[i].clamp(-1.0, 1.0) * amplitude).round() as i32;

        writer
            .write_sample(l_sample)
            .map_err(|e| format!("Error writing sample {i} (left): {e}"))?;
        writer
            .write_sample(r_sample)
            .map_err(|e| format!("Error writing sample {i} (right): {e}"))?;
    }

    writer
        .finalize()
        .map_err(|e| format!("Failed to finalize WAV file: {e}"))?;

    Ok(())
}

/// Returns the user's default recordings folder.
/// macOS: `~/Music/MiniStation/`
pub fn get_default_recordings_dir() -> PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        #[cfg(target_os = "macos")]
        {
            return PathBuf::from(home).join("Music").join("MiniStation");
        }
        #[cfg(not(target_os = "macos"))]
        {
            return PathBuf::from(home).join("Documents").join("MiniStation");
        }
    }

    if let Ok(userprofile) = std::env::var("USERPROFILE") {
        return PathBuf::from(userprofile).join("Music").join("MiniStation");
    }

    PathBuf::from("recordings")
}

/// Generates a standardized timestamped filename for loop exports.
/// e.g. `MiniStation_Loop_20261016_211500_120BPM.wav`
pub fn generate_timestamped_filename(bpm: f32) -> String {
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let bpm_int = bpm.round() as u32;
    format!("MiniStation_Loop_{now}_{bpm_int}BPM.wav")
}

/// Export the current active loop session (Track 1 + Track 2 summed) to a 24-bit WAV file.
pub fn export_looper_mix(
    looper: &LooperEngine,
    custom_path: Option<PathBuf>,
) -> Result<PathBuf, String> {
    let (mix_l, mix_r) = looper.get_mix_buffers();

    // Verify at least some audio has been recorded
    let has_signal = mix_l.iter().chain(mix_r.iter()).any(|&s| s.abs() > 0.0001);
    if !has_signal {
        return Err("No recorded loop audio found to export.".to_string());
    }

    let target_path = match custom_path {
        Some(p) => p,
        None => {
            let dir = get_default_recordings_dir();
            let filename = generate_timestamped_filename(looper.bpm);
            dir.join(filename)
        }
    };

    let sample_rate = looper.sample_rate as u32;
    export_to_wav(&mix_l, &mix_r, sample_rate, &target_path)?;

    Ok(target_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_export_to_wav_file_validity() {
        let temp_dir = std::env::temp_dir();
        let target_file = temp_dir.join("test_ministation_export.wav");

        let len = 44100; // 1 second
        let mut left = vec![0.0f32; len];
        let mut right = vec![0.0f32; len];

        // 440 Hz test sine wave
        for i in 0..len {
            let phase = i as f32 * 440.0 * 2.0 * std::f32::consts::PI / 44100.0;
            left[i] = phase.sin() * 0.7;
            right[i] = (phase + 0.5).sin() * 0.7;
        }

        let res = export_to_wav(&left, &right, 44100, &target_file);
        assert!(res.is_ok(), "WAV export must succeed: {:?}", res);

        assert!(target_file.exists());
        let meta = fs::metadata(&target_file).expect("File metadata must exist");
        // 1 sec of 24-bit stereo at 44100 Hz = 44100 * 2 channels * 3 bytes = 264,600 bytes + 44 bytes header
        assert!(meta.len() >= 264_600);

        // Read back with hound to verify standard compliance
        let reader = hound::WavReader::open(&target_file).expect("Must open with hound reader");
        let spec = reader.spec();
        assert_eq!(spec.channels, 2);
        assert_eq!(spec.sample_rate, 44100);
        assert_eq!(spec.bits_per_sample, 24);

        // Clean up
        let _ = fs::remove_file(target_file);
    }

    #[test]
    fn test_export_looper_mix_empty_rejection() {
        let looper = LooperEngine::new(44100.0);
        // Calling export on empty looper without audio must return informative error
        let res = export_looper_mix(&looper, None);
        assert!(res.is_err());
    }
}
