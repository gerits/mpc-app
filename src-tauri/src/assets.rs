use std::fs::{self, File};
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

/// High-level categories for instrument presets and browsing in the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InstrumentCategory {
    Piano,
    ElectricPiano,
    Synth,
    Bass,
    Drums,
    Other,
}

/// Metadata describing an individual preset/patch inside a SoundFont file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SoundFontPresetInfo {
    pub bank: u16,
    pub patch: u16,
    pub name: String,
}

/// Complete catalog metadata for a discovered SoundFont file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoundFontMetadata {
    pub id: String,
    pub name: String,
    pub filename: String,
    pub path: String,
    pub size_bytes: u64,
    pub category: InstrumentCategory,
    pub is_drum_kit: bool,
    pub is_user_provided: bool,
    pub author: Option<String>,
    pub copyright: Option<String>,
    pub presets_count: usize,
    pub presets: Vec<SoundFontPresetInfo>,
}

/// High-level curated quick-select performance preset configured out of the box.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CuratedPreset {
    pub id: String,
    pub display_name: String,
    pub category: InstrumentCategory,
    pub soundfont_filename: String,
    pub bank: u16,
    pub patch: u16,
    pub is_drum: bool,
    pub description: String,
}

/// Returns candidate directory paths where bundled soundfonts may be located.
pub fn get_bundled_soundfonts_dir() -> PathBuf {
    if let Ok(custom) = std::env::var("MINISTATION_ASSETS_DIR") {
        let p = PathBuf::from(custom);
        if p.exists() {
            return p;
        }
    }

    // Try relative to workspace / current working directory
    let candidates = [
        PathBuf::from("assets/soundfonts"),
        PathBuf::from("src-tauri/assets/soundfonts"),
        PathBuf::from("../src-tauri/assets/soundfonts"),
    ];

    for candidate in &candidates {
        if candidate.exists() && candidate.is_dir() {
            return candidate.clone();
        }
    }

    // Try relative to current executable
    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            let app_resources = exe_dir.join("../Resources/assets/soundfonts");
            if app_resources.exists() {
                return app_resources;
            }
            let exe_assets = exe_dir.join("assets/soundfonts");
            if exe_assets.exists() {
                return exe_assets;
            }
        }
    }

    // Default fallback
    PathBuf::from("src-tauri/assets/soundfonts")
}

/// Returns the persistent user directory for dropping custom `.sf2` soundfonts.
/// macOS: `~/Library/Application Support/MiniStation/soundfonts/`
/// Linux: `~/.local/share/MiniStation/soundfonts/`
/// Windows: `%APPDATA%/MiniStation/soundfonts/`
pub fn get_user_soundfonts_dir() -> PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        #[cfg(target_os = "macos")]
        {
            return PathBuf::from(home)
                .join("Library")
                .join("Application Support")
                .join("MiniStation")
                .join("soundfonts");
        }
        #[cfg(not(target_os = "macos"))]
        {
            return PathBuf::from(home)
                .join(".local")
                .join("share")
                .join("MiniStation")
                .join("soundfonts");
        }
    }

    if let Ok(appdata) = std::env::var("APPDATA") {
        return PathBuf::from(appdata)
            .join("MiniStation")
            .join("soundfonts");
    }

    PathBuf::from(".ministation").join("soundfonts")
}

/// Ensures the user soundfonts folder exists and returns its canonical path.
pub fn ensure_user_soundfonts_dir() -> std::io::Result<PathBuf> {
    let dir = get_user_soundfonts_dir();
    if !dir.exists() {
        fs::create_dir_all(&dir)?;
    }
    Ok(dir)
}

/// Inspect a single `.sf2` file and extract its complete metadata.
pub fn scan_soundfont_file<P: AsRef<Path>>(
    path: P,
    is_user: bool,
) -> Result<SoundFontMetadata, String> {
    let path = path.as_ref();
    let metadata = fs::metadata(path).map_err(|e| format!("Failed to read file metadata: {e}"))?;
    let size_bytes = metadata.len();

    let filename = path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown.sf2".to_string());

    let mut file = File::open(path).map_err(|e| format!("Failed to open {filename}: {e}"))?;
    let sf = rustysynth::SoundFont::new(&mut file)
        .map_err(|e| format!("Failed to parse SoundFont {filename}: {e:?}"))?;

    let id = filename
        .to_lowercase()
        .replace(".sf2", "")
        .replace(|c: char| !c.is_alphanumeric() && c != '_', "_");

    let bank_name = sf.get_info().get_bank_name().trim();
    let name = if !bank_name.is_empty() {
        bank_name.to_string()
    } else {
        filename
            .replace(".sf2", "")
            .replace(".SF2", "")
            .replace('_', " ")
    };

    let author = {
        let a = sf.get_info().get_author().trim();
        if a.is_empty() {
            None
        } else {
            Some(a.to_string())
        }
    };

    let copyright = {
        let c = sf.get_info().get_copyright().trim();
        if c.is_empty() {
            None
        } else {
            Some(c.to_string())
        }
    };

    let mut presets = Vec::new();
    let mut has_percussion_bank = false;

    for p in sf.get_presets() {
        let bank = p.get_bank_number() as u16;
        let patch = p.get_patch_number() as u16;
        if bank >= 128 {
            has_percussion_bank = true;
        }
        presets.push(SoundFontPresetInfo {
            bank,
            patch,
            name: p.get_name().trim().to_string(),
        });
    }

    let lower = filename.to_lowercase();
    let is_drum_kit = has_percussion_bank
        || lower.contains("drum")
        || lower.contains("909")
        || lower.contains("808")
        || lower.contains("perc");

    let category = if is_drum_kit {
        InstrumentCategory::Drums
    } else if lower.contains("piano") {
        InstrumentCategory::Piano
    } else if lower.contains("rhodes") || lower.contains("ep") || lower.contains("electric") {
        InstrumentCategory::ElectricPiano
    } else if lower.contains("bass") {
        InstrumentCategory::Bass
    } else if lower.contains("synth") || lower.contains("lead") || lower.contains("pad") {
        InstrumentCategory::Synth
    } else {
        InstrumentCategory::Other
    };

    Ok(SoundFontMetadata {
        id,
        name,
        filename,
        path: path.to_string_lossy().to_string(),
        size_bytes,
        category,
        is_drum_kit,
        is_user_provided: is_user,
        author,
        copyright,
        presets_count: presets.len(),
        presets,
    })
}

/// Scan a specific directory for `.sf2` and `.SF2` soundfont files.
pub fn scan_directory<P: AsRef<Path>>(dir: P, is_user: bool) -> Vec<SoundFontMetadata> {
    let dir = dir.as_ref();
    let mut result = Vec::new();

    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return result,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            if let Some(ext) = path.extension() {
                let ext_str = ext.to_string_lossy().to_lowercase();
                if ext_str == "sf2" {
                    if let Ok(meta) = scan_soundfont_file(&path, is_user) {
                        result.push(meta);
                    }
                }
            }
        }
    }

    result.sort_by(|a, b| a.name.cmp(&b.name));
    result
}

/// Scans both bundled application assets and user directories, merging discoveries.
pub fn scan_all_soundfonts() -> Vec<SoundFontMetadata> {
    let mut all = Vec::new();

    let bundled_dir = get_bundled_soundfonts_dir();
    if bundled_dir.exists() {
        all.extend(scan_directory(&bundled_dir, false));
    }

    let user_dir = get_user_soundfonts_dir();
    if user_dir.exists() {
        all.extend(scan_directory(&user_dir, true));
    }

    all
}

/// Returns the standard curated out-of-the-box presets mapped to the bundled soundfonts.
pub fn get_curated_presets() -> Vec<CuratedPreset> {
    vec![
        // Melodic: Acoustic Piano
        CuratedPreset {
            id: "grand-piano".to_string(),
            display_name: "Concert Grand Piano".to_string(),
            category: InstrumentCategory::Piano,
            soundfont_filename: "General_GM.sf2".to_string(),
            bank: 0,
            patch: 0,
            is_drum: false,
            description: "Warm, resonant full concert grand piano.".to_string(),
        },
        CuratedPreset {
            id: "acoustic-piano-bright".to_string(),
            display_name: "Bright Acoustic Grand".to_string(),
            category: InstrumentCategory::Piano,
            soundfont_filename: "Acoustic_Grand.sf2".to_string(),
            bank: 0,
            patch: 0,
            is_drum: false,
            description: "Crisp, defined acoustic piano with high attack clarity.".to_string(),
        },
        // Melodic: Electric Piano / Keys
        CuratedPreset {
            id: "rhodes-ep".to_string(),
            display_name: "Vintage Rhodes MK I".to_string(),
            category: InstrumentCategory::ElectricPiano,
            soundfont_filename: "General_GM.sf2".to_string(),
            bank: 0,
            patch: 4,
            is_drum: false,
            description: "Warm vintage tine electric piano with natural bell harmonics.".to_string(),
        },
        CuratedPreset {
            id: "fm-electric-piano".to_string(),
            display_name: "FM Electric Piano".to_string(),
            category: InstrumentCategory::ElectricPiano,
            soundfont_filename: "General_GM.sf2".to_string(),
            bank: 0,
            patch: 5,
            is_drum: false,
            description: "Classic 80s crystal FM synthesizer digital piano.".to_string(),
        },
        // Melodic: Bass
        CuratedPreset {
            id: "acoustic-upright-bass".to_string(),
            display_name: "Acoustic Upright Bass".to_string(),
            category: InstrumentCategory::Bass,
            soundfont_filename: "General_GM.sf2".to_string(),
            bank: 0,
            patch: 32,
            is_drum: false,
            description: "Round, woody acoustic double bass.".to_string(),
        },
        CuratedPreset {
            id: "electric-finger-bass".to_string(),
            display_name: "Electric Finger Bass".to_string(),
            category: InstrumentCategory::Bass,
            soundfont_filename: "General_GM.sf2".to_string(),
            bank: 0,
            patch: 33,
            is_drum: false,
            description: "Deep, punchy 4-string electric bass guitar.".to_string(),
        },
        CuratedPreset {
            id: "moog-synth-bass".to_string(),
            display_name: "Analog Synth Bass".to_string(),
            category: InstrumentCategory::Bass,
            soundfont_filename: "General_GM.sf2".to_string(),
            bank: 0,
            patch: 38,
            is_drum: false,
            description: "Heavy, resonant analog filter synth bass for modern beats.".to_string(),
        },
        // Melodic: Synth Leads & Pads
        CuratedPreset {
            id: "analog-saw-lead".to_string(),
            display_name: "Analog Saw Lead".to_string(),
            category: InstrumentCategory::Synth,
            soundfont_filename: "General_GM.sf2".to_string(),
            bank: 0,
            patch: 81,
            is_drum: false,
            description: "Cutting saw wave synthesizer lead with rich harmonics.".to_string(),
        },
        CuratedPreset {
            id: "square-solo-lead".to_string(),
            display_name: "Square Solo Lead".to_string(),
            category: InstrumentCategory::Synth,
            soundfont_filename: "General_GM.sf2".to_string(),
            bank: 0,
            patch: 80,
            is_drum: false,
            description: "Classic chiptune-inspired square wave lead.".to_string(),
        },
        CuratedPreset {
            id: "warm-analog-pad".to_string(),
            display_name: "Warm Analog Pad".to_string(),
            category: InstrumentCategory::Synth,
            soundfont_filename: "General_GM.sf2".to_string(),
            bank: 0,
            patch: 89,
            is_drum: false,
            description: "Lush, slow-attack polyphonic ambient pad.".to_string(),
        },
        // Drum Kits for MPC Pads (Channel 10, Notes 36–51)
        CuratedPreset {
            id: "tr-808-kit".to_string(),
            display_name: "Roland TR-808 Kit".to_string(),
            category: InstrumentCategory::Drums,
            soundfont_filename: "General_GM.sf2".to_string(),
            bank: 128,
            patch: 25,
            is_drum: true,
            description: "Legendary deep sub kick, snappy snare, and classic claps.".to_string(),
        },
        CuratedPreset {
            id: "tr-909-kit".to_string(),
            display_name: "Roland TR-909 Kit".to_string(),
            category: InstrumentCategory::Drums,
            soundfont_filename: "909_drums.sf2".to_string(),
            bank: 0,
            patch: 0,
            is_drum: true,
            description: "Punchy techno/house staple kick, snare, toms, and hi-hats.".to_string(),
        },
        CuratedPreset {
            id: "studio-drums".to_string(),
            display_name: "Acoustic Studio Kit".to_string(),
            category: InstrumentCategory::Drums,
            soundfont_filename: "General_GM.sf2".to_string(),
            bank: 128,
            patch: 0,
            is_drum: true,
            description: "Natural acoustic wood kick, maple snare, and crisp cymbals.".to_string(),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bundled_soundfonts_discovery() {
        let bundled = get_bundled_soundfonts_dir();
        assert!(bundled.exists(), "Bundled soundfonts dir must exist: {:?}", bundled);

        let soundfonts = scan_directory(&bundled, false);
        assert!(
            !soundfonts.is_empty(),
            "Must discover bundled soundfonts in {:?}",
            bundled
        );

        let filenames: Vec<&str> = soundfonts.iter().map(|s| s.filename.as_str()).collect();
        assert!(filenames.contains(&"General_GM.sf2") || filenames.contains(&"909_drums.sf2"));
    }

    #[test]
    fn test_scan_all_soundfonts_returns_presets() {
        let all = scan_all_soundfonts();
        assert!(!all.is_empty());

        let gm = all.iter().find(|s| s.filename == "General_GM.sf2");
        assert!(gm.is_some());
        let gm_meta = gm.unwrap();
        assert!(gm_meta.presets_count > 100);
    }

    #[test]
    fn test_curated_presets_list() {
        let presets = get_curated_presets();
        assert!(presets.len() >= 10);

        let p_piano = presets.iter().find(|p| p.id == "grand-piano");
        assert!(p_piano.is_some());
        assert_eq!(p_piano.unwrap().category, InstrumentCategory::Piano);

        let p_808 = presets.iter().find(|p| p.id == "tr-808-kit");
        assert!(p_808.is_some());
        assert!(p_808.unwrap().is_drum);
    }
}
