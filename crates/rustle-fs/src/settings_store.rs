//! Read and write `settings.toml` on disk.

use std::path::{Path, PathBuf};

use rustle_core::{FsError, Settings};

/// `~/.config/rustle/settings.toml` on Linux (the OS config folder elsewhere).
pub fn settings_path() -> PathBuf {
    dirs::config_dir()
        .or_else(dirs::home_dir)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("rustle")
        .join("settings.toml")
}

/// Load settings. This never fails: if the file is missing or broken, you
/// get the defaults, so a bad file can't stop Rustle from starting.
pub fn load_settings(path: &Path) -> Settings {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|text| toml::from_str::<Settings>(&text).ok())
        .unwrap_or_default()
        .normalized()
}

/// Save settings. Writes to a temporary file first and then renames it over
/// the real one, so a crash in the middle never leaves a half-written file.
pub fn save_settings(path: &Path, settings: &Settings) -> Result<(), FsError> {
    let text = toml::to_string_pretty(settings).map_err(|e| FsError::Other(e.to_string()))?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| FsError::Other(e.to_string()))?;
    }
    let tmp = path.with_extension("toml.tmp");
    std::fs::write(&tmp, text).map_err(|e| FsError::Other(e.to_string()))?;
    std::fs::rename(&tmp, path).map_err(|e| FsError::Other(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustle_core::ThemeChoice;

    #[test]
    fn round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("x/settings.toml");
        let s = Settings {
            theme: ThemeChoice::Light,
            opacity: 0.8,
            blur: true,
            show_hidden: true,
            favorites: vec!["/tmp".into()],
        };
        save_settings(&p, &s).unwrap();
        assert_eq!(load_settings(&p), s);
    }

    #[test]
    fn missing_file_gives_defaults() {
        assert_eq!(
            load_settings(Path::new("/nonexistent/s.toml")),
            Settings::default()
        );
    }

    #[test]
    fn corrupt_file_gives_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("s.toml");
        std::fs::write(&p, "theme = [[[ not toml").unwrap();
        assert_eq!(load_settings(&p), Settings::default());
    }

    #[test]
    fn partial_file_fills_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("s.toml");
        std::fs::write(&p, "opacity = 0.7\n").unwrap();
        let s = load_settings(&p);
        assert_eq!(s.opacity, 0.7);
        assert_eq!(s.theme, ThemeChoice::Dark);
    }
}
