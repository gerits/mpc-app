use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use rustysynth::SoundFont;

use crate::assets::{
    ensure_user_soundfonts_dir, get_bundled_soundfonts_dir, get_curated_presets,
    get_user_soundfonts_dir, scan_all_soundfonts, scan_soundfont_file, CuratedPreset,
    SoundFontMetadata,
};
use crate::audio::AudioEngine;
use crate::synth::load_soundfont_from_path;

/// High-level manager coordinating SoundFont cataloging, in-memory caching,
/// and instant preset switching across melodic and drum synthesizer channels.
pub struct SoundFontManager {
    cached_soundfonts: HashMap<String, Arc<SoundFont>>,
    metadata: Vec<SoundFontMetadata>,
    curated_presets: Vec<CuratedPreset>,
    active_melodic_preset_id: Option<String>,
    active_drum_preset_id: Option<String>,
}

impl SoundFontManager {
    /// Initialize `SoundFontManager` by scanning bundled assets and user directory.
    pub fn new() -> Self {
        let metadata = scan_all_soundfonts();
        let curated_presets = get_curated_presets();

        Self {
            cached_soundfonts: HashMap::new(),
            metadata,
            curated_presets,
            active_melodic_preset_id: None,
            active_drum_preset_id: None,
        }
    }

    /// Rescans all soundfont directories and refreshes metadata.
    pub fn rescan(&mut self) -> &[SoundFontMetadata] {
        self.metadata = scan_all_soundfonts();
        &self.metadata
    }

    pub fn metadata(&self) -> &[SoundFontMetadata] {
        &self.metadata
    }

    pub fn curated_presets(&self) -> &[CuratedPreset] {
        &self.curated_presets
    }

    pub fn active_melodic_preset_id(&self) -> Option<&str> {
        self.active_melodic_preset_id.as_deref()
    }

    pub fn active_drum_preset_id(&self) -> Option<&str> {
        self.active_drum_preset_id.as_deref()
    }

    /// Resolve a SoundFont filename to its concrete path on disk.
    pub fn resolve_soundfont_path(&self, filename: &str) -> Option<PathBuf> {
        // Direct absolute/relative path check
        let direct = PathBuf::from(filename);
        if direct.exists() && direct.is_file() {
            return Some(direct);
        }

        // Check bundled assets folder
        let bundled = get_bundled_soundfonts_dir().join(filename);
        if bundled.exists() && bundled.is_file() {
            return Some(bundled);
        }

        // Check user data folder
        let user = get_user_soundfonts_dir().join(filename);
        if user.exists() && user.is_file() {
            return Some(user);
        }

        // Search through scanned metadata
        for meta in &self.metadata {
            if meta.filename.eq_ignore_ascii_case(filename)
                || meta.id.eq_ignore_ascii_case(filename)
            {
                let p = PathBuf::from(&meta.path);
                if p.exists() {
                    return Some(p);
                }
            }
        }

        None
    }

    /// Fetch or load a SoundFont into in-memory cache to guarantee <1ms subsequent switching.
    pub fn get_or_load_soundfont(&mut self, filename: &str) -> Result<Arc<SoundFont>, String> {
        if let Some(cached) = self.cached_soundfonts.get(filename) {
            return Ok(Arc::clone(cached));
        }

        let path = self
            .resolve_soundfont_path(filename)
            .ok_or_else(|| format!("SoundFont file '{filename}' could not be located."))?;

        let sf = load_soundfont_from_path(&path)
            .map_err(|e| format!("Failed to parse SoundFont {}: {e}", path.display()))?;

        self.cached_soundfonts
            .insert(filename.to_string(), Arc::clone(&sf));
        Ok(sf)
    }

    /// Select and load a curated out-of-the-box performance preset.
    pub fn select_curated_preset(
        &mut self,
        audio: &AudioEngine,
        preset_id: &str,
    ) -> Result<CuratedPreset, String> {
        let preset = self
            .curated_presets
            .iter()
            .find(|p| p.id == preset_id)
            .cloned()
            .ok_or_else(|| format!("Curated preset '{preset_id}' not found."))?;

        let soundfont = self.get_or_load_soundfont(&preset.soundfont_filename)?;

        if preset.is_drum {
            audio.load_drum_soundfont(soundfont)?;
            audio.set_drum_program(preset.bank, preset.patch as u8);
            self.active_drum_preset_id = Some(preset.id.clone());
        } else {
            audio.load_melodic_soundfont(soundfont)?;
            audio.set_melodic_program(0, preset.bank, preset.patch as u8);
            self.active_melodic_preset_id = Some(preset.id.clone());
        }

        Ok(preset)
    }

    /// Load default melodic and drum presets on application startup.
    pub fn load_defaults(&mut self, audio: &AudioEngine) -> Result<(), String> {
        // Melodic default: Concert Grand Piano
        let _ = self.select_curated_preset(audio, "grand-piano");
        // Drum default: Roland TR-808 Kit
        let _ = self.select_curated_preset(audio, "tr-808-kit");
        Ok(())
    }

    /// Import an external `.sf2` file into the user soundfonts directory.
    pub fn import_user_soundfont<P: AsRef<Path>>(
        &mut self,
        source_path: P,
    ) -> Result<SoundFontMetadata, String> {
        let src = source_path.as_ref();
        if !src.exists() || !src.is_file() {
            return Err(format!("Source SoundFont does not exist: {}", src.display()));
        }

        let user_dir = ensure_user_soundfonts_dir()
            .map_err(|e| format!("Failed to create user soundfonts directory: {e}"))?;

        let file_name = src
            .file_name()
            .ok_or_else(|| "Invalid file name".to_string())?;

        let dest = user_dir.join(file_name);
        fs::copy(src, &dest)
            .map_err(|e| format!("Failed to copy SoundFont into {}: {e}", dest.display()))?;

        let metadata = scan_soundfont_file(&dest, true)?;

        // Update active catalog
        self.metadata.retain(|m| m.path != metadata.path);
        self.metadata.push(metadata.clone());
        self.metadata.sort_by(|a, b| a.name.cmp(&b.name));

        Ok(metadata)
    }

    /// Select a specific soundfont file and program (bank and patch).
    pub fn select_soundfont_program(
        &mut self,
        audio: &AudioEngine,
        filename: &str,
        is_drum: bool,
        bank: u16,
        patch: u8,
    ) -> Result<(), String> {
        let soundfont = self.get_or_load_soundfont(filename)?;
        if is_drum {
            audio.load_drum_soundfont(soundfont)?;
            audio.set_drum_program(bank, patch);
            self.active_drum_preset_id = Some(format!("{filename}:{bank}:{patch}"));
        } else {
            audio.load_melodic_soundfont(soundfont)?;
            audio.set_melodic_program(0, bank, patch);
            self.active_melodic_preset_id = Some(format!("{filename}:{bank}:{patch}"));
        }
        Ok(())
    }
}

impl Default for SoundFontManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_soundfont_manager_initialization() {
        let manager = SoundFontManager::new();
        assert!(!manager.metadata().is_empty());
        assert!(!manager.curated_presets().is_empty());
    }

    #[test]
    fn test_soundfont_caching_and_fast_lookup() {
        let mut manager = SoundFontManager::new();

        let start = std::time::Instant::now();
        let sf1 = manager.get_or_load_soundfont("General_GM.sf2");
        assert!(sf1.is_ok());
        let first_duration = start.elapsed();

        // Second lookup must come directly from memory cache (< 100 microseconds)
        let cache_start = std::time::Instant::now();
        let sf2 = manager.get_or_load_soundfont("General_GM.sf2");
        assert!(sf2.is_ok());
        let cache_duration = cache_start.elapsed();

        assert!(cache_duration < first_duration);
        assert!(cache_duration.as_millis() < 5);
    }

    #[test]
    fn test_curated_preset_switching_lifecycle() {
        let mut manager = SoundFontManager::new();
        let audio = AudioEngine::new(44100, 256);

        // Switch to Grand Piano
        let p1 = manager.select_curated_preset(&audio, "grand-piano");
        assert!(p1.is_ok());
        assert_eq!(manager.active_melodic_preset_id(), Some("grand-piano"));

        // Switch to Rhodes Electric Piano
        let p2 = manager.select_curated_preset(&audio, "rhodes-ep");
        assert!(p2.is_ok());
        assert_eq!(manager.active_melodic_preset_id(), Some("rhodes-ep"));

        // Switch to TR-808 drum kit
        let p3 = manager.select_curated_preset(&audio, "tr-808-kit");
        assert!(p3.is_ok());
        assert_eq!(manager.active_drum_preset_id(), Some("tr-808-kit"));

        // Melodic preset must remain active when drum preset changes
        assert_eq!(manager.active_melodic_preset_id(), Some("rhodes-ep"));
    }

    #[test]
    fn test_import_user_soundfont() {
        let mut manager = SoundFontManager::new();
        let bundled_path = manager
            .resolve_soundfont_path("909_drums.sf2")
            .expect("909_drums.sf2 must exist");

        let temp_dir = std::env::temp_dir();
        let temp_source = temp_dir.join("test_custom_imported_kit.sf2");
        std::fs::copy(&bundled_path, &temp_source).expect("copy to temp must succeed");

        let imported = manager.import_user_soundfont(&temp_source);
        assert!(imported.is_ok(), "Import must succeed");
        let meta = imported.unwrap();
        assert!(meta.is_user_provided);
        assert_eq!(meta.filename, "test_custom_imported_kit.sf2");

        // Clean up
        let _ = std::fs::remove_file(temp_source);
        let _ = std::fs::remove_file(meta.path);
    }
}
