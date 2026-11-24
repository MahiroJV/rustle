//! User settings. Only the data and its rules live here; reading and writing
//! the file is the job of `rustle-fs`.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::view::ViewSettings;

pub const MIN_OPACITY: f32 = 0.60;
pub const MAX_OPACITY: f32 = 1.00;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemeChoice {
    #[default]
    Dark,
    Light,
}

/// Everything the user can change on the settings page.
///
/// `#[serde(default)]` means a field missing from the file gets its value
/// from `Settings::default()`, so old or hand-edited files still load.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub theme: ThemeChoice,
    /// Window opacity, from `MIN_OPACITY` to `MAX_OPACITY`.
    pub opacity: f32,
    /// Blur behind the window. Applied when the window opens.
    pub blur: bool,
    pub show_hidden: bool,
    pub favorites: Vec<PathBuf>,
    /// Layout, icon size and sort order of the file list.
    pub view: ViewSettings,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: ThemeChoice::Dark,
            opacity: MAX_OPACITY,
            blur: false,
            show_hidden: false,
            favorites: Vec::new(),
            view: ViewSettings::default(),
        }
    }
}

impl Settings {
    /// Fix values that could have come from a bad or hand-edited file:
    /// opacity into range, and no duplicate favorites (first one wins).
    pub fn normalized(mut self) -> Self {
        self.opacity = clamp_opacity(self.opacity);
        let mut seen = std::collections::HashSet::new();
        self.favorites.retain(|path| seen.insert(path.clone()));
        self
    }
}

/// Keep opacity in range. NaN (not a number) becomes fully opaque.
pub fn clamp_opacity(value: f32) -> f32 {
    if value.is_nan() {
        MAX_OPACITY
    } else {
        value.clamp(MIN_OPACITY, MAX_OPACITY)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults() {
        let s = Settings::default();
        assert_eq!(
            (s.theme, s.opacity, s.blur, s.show_hidden),
            (ThemeChoice::Dark, 1.0, false, false)
        );
        assert!(s.favorites.is_empty());
    }

    #[test]
    fn opacity_is_clamped() {
        assert_eq!(clamp_opacity(0.2), 0.60);
        assert_eq!(clamp_opacity(1.5), 1.00);
        assert_eq!(clamp_opacity(f32::NAN), 1.0);
        assert_eq!(clamp_opacity(0.8), 0.8);
    }

    #[test]
    fn normalized_dedups_favorites_keeping_order() {
        let s = Settings {
            favorites: vec!["/a".into(), "/b".into(), "/a".into()],
            opacity: 9.0,
            ..Default::default()
        }
        .normalized();
        assert_eq!(s.favorites, vec![PathBuf::from("/a"), PathBuf::from("/b")]);
        assert_eq!(s.opacity, 1.0);
    }
}
