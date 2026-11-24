//! Picture thumbnails, saved on disk so each picture is only shrunk once.
//!
//! Cache: `~/.cache/rustle/thumbs/<key>.png`. The key mixes the picture's
//! path, size and modified time, so an edited picture gets a new thumbnail
//! automatically (old ones are simply never read again).

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use image::ImageFormat;
use rustle_core::{FsError, extension};

/// Pictures bigger than this aren't previewed (too slow to decode).
pub const MAX_PREVIEW_BYTES: u64 = 50 * 1024 * 1024;
/// Thumbnails are made once at this size and scaled down for display.
pub const THUMB_PX: u32 = 256;

/// A decoded thumbnail: raw RGBA pixels, ready to hand to the UI.
#[derive(Debug, Clone)]
pub struct Thumb {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// Can we make a preview for this file? Decided by extension and size only,
/// without opening it.
pub fn is_previewable(name: &str, len: u64) -> bool {
    let supported = matches!(
        extension(name).as_deref(),
        Some("png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp")
    );
    supported && len <= MAX_PREVIEW_BYTES
}

pub fn thumbs_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("rustle")
        .join("thumbs")
}

/// The thumbnail for `path`, from the cache in `thumbs_dir()` or freshly made.
pub fn thumbnail(path: &Path) -> Result<Thumb, FsError> {
    thumbnail_in(&thumbs_dir(), path)
}

/// Same as `thumbnail`, with the cache folder as a parameter (for tests).
pub fn thumbnail_in(cache_dir: &Path, path: &Path) -> Result<Thumb, FsError> {
    let err = |e: &dyn std::fmt::Display| FsError::Other(format!("{}: {e}", path.display()));

    let meta = std::fs::metadata(path).map_err(|e| err(&e))?;
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    if !is_previewable(&name, meta.len()) {
        return Err(FsError::Other(format!("no preview for {}", path.display())));
    }

    let cached = cache_dir.join(format!("{}.png", cache_key(path, meta.len(), meta.modified().ok())));
    // Cache hit: the small PNG decodes much faster than the original picture.
    if let Ok(img) = image::open(&cached) {
        return Ok(to_thumb(img));
    }

    // Cache miss: decode the original, shrink it (keeping its shape), save it.
    let original = image::open(path).map_err(|e| err(&e))?;
    // `thumbnail()` would also *enlarge* small pictures (blurry), so only
    // shrink pictures that are bigger than the thumbnail size.
    let small = if original.width() > THUMB_PX || original.height() > THUMB_PX {
        original.thumbnail(THUMB_PX, THUMB_PX)
    } else {
        original
    };
    if std::fs::create_dir_all(cache_dir).is_ok() {
        // Write to a temporary name, then rename: another Rustle window
        // reading the cache never sees a half-written file. A failed cache
        // write is fine: we still return the thumbnail, just uncached.
        let tmp = cached.with_extension("png.tmp");
        if small.save_with_format(&tmp, ImageFormat::Png).is_ok() {
            let _ = std::fs::rename(&tmp, &cached);
        }
    }
    Ok(to_thumb(small))
}

fn to_thumb(img: image::DynamicImage) -> Thumb {
    let rgba = img.into_rgba8();
    Thumb {
        width: rgba.width(),
        height: rgba.height(),
        rgba: rgba.into_raw(),
    }
}

/// A short, stable file name for the cache. Uses FNV-1a, a tiny hash that
/// gives the same result in every Rust version (std's `DefaultHasher` doesn't
/// promise that, and a changed hash would throw the whole cache away).
pub fn cache_key(path: &Path, len: u64, modified: Option<SystemTime>) -> String {
    let nanos = modified
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_nanos());
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325; // FNV offset basis
    let mut feed = |bytes: &[u8]| {
        for b in bytes {
            hash ^= u64::from(*b);
            hash = hash.wrapping_mul(0x0100_0000_01b3); // FNV prime
        }
    };
    feed(path.as_os_str().as_encoded_bytes());
    feed(&len.to_le_bytes());
    feed(&nanos.to_le_bytes());
    format!("{hash:016x}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn make_png(path: &Path, w: u32, h: u32) {
        image::RgbaImage::from_pixel(w, h, image::Rgba([200, 100, 50, 255]))
            .save(path)
            .unwrap();
    }

    #[test]
    fn previewable_by_extension_and_size() {
        assert!(is_previewable("cat.JPG", 1000));
        assert!(is_previewable("x.webp", 1000));
        assert!(!is_previewable("notes.txt", 10));
        assert!(!is_previewable("huge.png", MAX_PREVIEW_BYTES + 1));
    }

    #[test]
    fn thumbnail_keeps_shape_and_is_cached() {
        let dir = tempfile::tempdir().unwrap();
        let (pic, cache) = (dir.path().join("wide.png"), dir.path().join("cache"));
        make_png(&pic, 600, 300);

        let t = thumbnail_in(&cache, &pic).unwrap();
        assert_eq!((t.width, t.height), (THUMB_PX, THUMB_PX / 2));
        assert_eq!(t.rgba.len(), (t.width * t.height * 4) as usize);
        // One cached file was written…
        assert_eq!(std::fs::read_dir(&cache).unwrap().count(), 1);
        // …and a second call gives the same result without adding more.
        let again = thumbnail_in(&cache, &pic).unwrap();
        assert_eq!((again.width, again.height), (t.width, t.height));
        assert_eq!(std::fs::read_dir(&cache).unwrap().count(), 1);
    }

    #[test]
    fn small_pictures_are_not_enlarged() {
        let dir = tempfile::tempdir().unwrap();
        let pic = dir.path().join("tiny.png");
        make_png(&pic, 40, 20);
        let t = thumbnail_in(&dir.path().join("c"), &pic).unwrap();
        assert_eq!((t.width, t.height), (40, 20));
    }

    #[test]
    fn changed_file_gets_a_new_key() {
        let p = Path::new("/pics/a.png");
        let t1 = Some(UNIX_EPOCH + Duration::from_secs(100));
        let t2 = Some(UNIX_EPOCH + Duration::from_secs(101));
        assert_eq!(cache_key(p, 10, t1), cache_key(p, 10, t1));
        assert_ne!(cache_key(p, 10, t1), cache_key(p, 10, t2)); // edited
        assert_ne!(cache_key(p, 10, t1), cache_key(p, 11, t1)); // resized
        assert_ne!(cache_key(p, 10, t1), cache_key(Path::new("/pics/b.png"), 10, t1));
    }

    #[test]
    fn broken_picture_is_an_error_not_a_crash() {
        let dir = tempfile::tempdir().unwrap();
        let pic = dir.path().join("broken.png");
        std::fs::write(&pic, b"this is not a png").unwrap();
        assert!(thumbnail_in(&dir.path().join("c"), &pic).is_err());
    }
}
