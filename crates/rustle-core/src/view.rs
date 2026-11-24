//! How the file list looks: layout, icon size and sort order.
//! Saved as part of `Settings`, so it's the same for every folder.

use serde::{Deserialize, Serialize};

use crate::entry::SortKey;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Layout {
    /// Rows with Name · Items · Size · Modified · Type columns.
    #[default]
    Details,
    /// Compact rows with only icon and name.
    List,
    /// A grid of big icons with the name underneath.
    Icons,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IconSize {
    Small,
    #[default]
    Medium,
    Large,
    ExtraLarge,
}

impl IconSize {
    pub const ALL: [IconSize; 4] = [
        IconSize::Small,
        IconSize::Medium,
        IconSize::Large,
        IconSize::ExtraLarge,
    ];

    /// One step bigger, stopping at the largest.
    pub fn bigger(self) -> Self {
        let i = self as usize;
        Self::ALL[(i + 1).min(Self::ALL.len() - 1)]
    }

    /// One step smaller, stopping at the smallest.
    pub fn smaller(self) -> Self {
        let i = self as usize;
        Self::ALL[i.saturating_sub(1)]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ViewSettings {
    pub layout: Layout,
    pub icon_size: IconSize,
    pub sort: SortKey,
    pub descending: bool,
}

impl ViewSettings {
    /// Clicking a column header: the same column flips the direction,
    /// a different column sorts by it ascending.
    pub fn click_column(&mut self, key: SortKey) {
        if self.sort == key {
            self.descending = !self.descending;
        } else {
            self.sort = key;
            self.descending = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zoom_stops_at_the_ends() {
        assert_eq!(IconSize::Medium.bigger(), IconSize::Large);
        assert_eq!(IconSize::ExtraLarge.bigger(), IconSize::ExtraLarge);
        assert_eq!(IconSize::Medium.smaller(), IconSize::Small);
        assert_eq!(IconSize::Small.smaller(), IconSize::Small);
    }

    #[test]
    fn clicking_a_column() {
        let mut v = ViewSettings::default();
        v.click_column(SortKey::Size);
        assert_eq!((v.sort, v.descending), (SortKey::Size, false));
        v.click_column(SortKey::Size);
        assert_eq!((v.sort, v.descending), (SortKey::Size, true));
        v.click_column(SortKey::Name);
        assert_eq!((v.sort, v.descending), (SortKey::Name, false));
    }
}
