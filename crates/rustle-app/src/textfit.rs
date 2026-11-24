//! Making long names fit: iced 0.14 can't add "…" by itself, so we shorten
//! the text before showing it.

/// How many characters of `font_px`-sized text roughly fit in `width_px`.
/// An estimate (letters have different widths), so it errs on the short
/// side; the cell also clips, so a wrong guess never draws over neighbors.
pub fn fit_chars(width_px: f32, font_px: f32) -> usize {
    /// An average letter is about 0.6 × the font size wide (a bit generous,
    /// so names are cut slightly early rather than too late).
    const LETTER_WIDTH: f32 = 0.6;
    const AT_LEAST: usize = 4;
    let n = (width_px / (font_px * LETTER_WIDTH)).floor();
    // A negative/NaN width gives a negative/NaN `n`; `as usize` turns those
    // into 0, and `max` makes it at least a few letters.
    (n as usize).max(AT_LEAST)
}

/// Shorten to at most `max` characters by replacing the middle with "…",
/// like GNOME Files. The end is kept, so the extension stays visible:
/// "Eclipse IDE for Java Developers - 2025-12.desktop" → "Eclipse IDE fo…2025-12.desktop"
pub fn shorten_middle(name: &str, max: usize) -> String {
    let count = name.chars().count();
    if count <= max {
        return name.to_string();
    }
    if max == 0 {
        return String::new();
    }
    // `max - 1` letters plus the "…". The end gets the bigger half, because
    // the end (version, extension) tells similar files apart.
    let keep = max - 1;
    let tail = keep.div_ceil(2);
    let head = keep - tail;
    // Work with `chars()` (letters), never byte positions: cutting a
    // multi-byte letter like "ş" in half would panic.
    let start: String = name.chars().take(head).collect();
    let end: String = name.chars().skip(count - tail).collect();
    format!("{start}…{end}")
}

#[cfg(test)]
mod tests {
    use super::*;

    const LONG: &str = "Eclipse IDE for Java Developers - 2025-12.desktop";

    #[test]
    fn short_names_are_unchanged() {
        assert_eq!(shorten_middle("steam.desktop", 30), "steam.desktop");
    }

    #[test]
    fn long_names_keep_start_and_end() {
        let s = shorten_middle(LONG, 30);
        assert_eq!(s.chars().count(), 30);
        assert!(s.starts_with("Eclipse IDE"), "{s}");
        assert!(s.ends_with("2025-12.desktop"), "{s}");
        assert!(s.contains('…'), "{s}");
    }

    #[test]
    fn counts_characters_not_bytes() {
        // Each "ş"/"ə" is 2 bytes; cutting by bytes could split a letter and panic.
        let s = shorten_middle("şəkillər-və-videolar-2026.png", 12);
        assert_eq!(s.chars().count(), 12);
        assert!(s.ends_with(".png"), "{s}");
    }

    #[test]
    fn tiny_limits_still_work() {
        assert_eq!(shorten_middle(LONG, 1), "…");
        assert_eq!(shorten_middle(LONG, 0), "");
    }

    #[test]
    fn width_estimate() {
        // 16 px text is about 9–10 px per letter: 300 px holds ~30 letters.
        let n = fit_chars(300.0, 16.0);
        assert!((25..=35).contains(&n), "{n}");
        // Never less than a few letters, even in a tiny space.
        assert_eq!(fit_chars(5.0, 16.0), 4);
        assert_eq!(fit_chars(-10.0, 16.0), 4);
    }
}
