use std::path::PathBuf;

/// One item in a directory listing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub size: u64,
    pub hidden: bool,
}

/// Folders first, then case-insensitive by name.
pub fn sort_entries(entries: &mut [Entry]) {
    // Cached key: each name is lowercased once, not on every comparison.
    entries.sort_by_cached_key(|e| (!e.is_dir, e.name.to_lowercase()));
}

/// Human-readable size, e.g. `512 B`, `1.5 KiB`, `3.2 MiB`.
pub fn format_size(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["KiB", "MiB", "GiB", "TiB", "PiB"];
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    let mut value = bytes as f64 / 1024.0;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    format!("{value:.1} {}", UNITS[unit])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn e(name: &str, is_dir: bool) -> Entry {
        Entry {
            name: name.into(),
            path: name.into(),
            is_dir,
            size: 0,
            hidden: false,
        }
    }

    #[test]
    fn folders_come_first_then_by_name() {
        let mut v = vec![
            e("b.txt", false),
            e("Zeta", true),
            e("a.txt", false),
            e("alpha", true),
        ];
        sort_entries(&mut v);
        let names: Vec<_> = v.iter().map(|x| x.name.as_str()).collect();
        assert_eq!(names, ["alpha", "Zeta", "a.txt", "b.txt"]);
    }

    #[test]
    fn sizes_are_human_readable() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(1023), "1023 B");
        assert_eq!(format_size(1024), "1.0 KiB");
        assert_eq!(format_size(1536), "1.5 KiB");
        assert_eq!(format_size(5 * 1024 * 1024), "5.0 MiB");
    }
}