//! What kind of file something is, judged by its extension.
//!
//! Used for the "Type" column, "sort by type", and as the fallback when
//! there's no specific icon for a file.

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FileKind {
    Folder,
    Image,
    Audio,
    Video,
    Archive,
    Code,
    Document,
    Text,
    App,
    Other,
}

impl FileKind {
    /// Human-readable name for the Type column.
    pub fn label(self) -> &'static str {
        match self {
            FileKind::Folder => "Folder",
            FileKind::Image => "Image",
            FileKind::Audio => "Audio",
            FileKind::Video => "Video",
            FileKind::Archive => "Archive",
            FileKind::Code => "Code",
            FileKind::Document => "Document",
            FileKind::Text => "Text",
            FileKind::App => "Application",
            FileKind::Other => "File",
        }
    }
}

/// The part after the last dot, lowercased. `None` for names without one,
/// and for dot-files like `.bashrc` (the dot there isn't an extension).
pub fn extension(name: &str) -> Option<String> {
    let (stem, ext) = name.rsplit_once('.')?;
    if stem.is_empty() || ext.is_empty() {
        None
    } else {
        Some(ext.to_ascii_lowercase())
    }
}

pub fn file_kind(name: &str, is_dir: bool) -> FileKind {
    if is_dir {
        return FileKind::Folder;
    }
    let Some(ext) = extension(name) else {
        return FileKind::Other;
    };
    match ext.as_str() {
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" | "svg" | "ico" | "tif" | "tiff"
        | "heic" | "avif" => FileKind::Image,
        "mp3" | "wav" | "flac" | "ogg" | "m4a" | "aac" | "opus" | "wma" => FileKind::Audio,
        "mp4" | "mkv" | "webm" | "avi" | "mov" | "wmv" | "flv" | "m4v" => FileKind::Video,
        "zip" | "tar" | "gz" | "tgz" | "bz2" | "xz" | "7z" | "rar" | "zst" | "deb" | "rpm"
        | "iso" => FileKind::Archive,
        "rs" | "py" | "js" | "ts" | "jsx" | "tsx" | "html" | "css" | "scss" | "c" | "h" | "cpp"
        | "hpp" | "cc" | "go" | "java" | "kt" | "swift" | "cs" | "rb" | "php" | "lua" | "sh"
        | "bash" | "zsh" | "fish" | "nix" | "toml" | "yaml" | "yml" | "json" | "xml" | "sql" => {
            FileKind::Code
        }
        "pdf" | "doc" | "docx" | "odt" | "rtf" | "xls" | "xlsx" | "ods" | "csv" | "ppt"
        | "pptx" | "odp" | "epub" => FileKind::Document,
        "txt" | "md" | "log" | "ini" | "conf" | "cfg" => FileKind::Text,
        "exe" | "msi" | "appimage" | "apk" | "dmg" => FileKind::App,
        _ => FileKind::Other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extensions() {
        assert_eq!(extension("photo.JPG"), Some("jpg".into()));
        assert_eq!(extension("a.tar.gz"), Some("gz".into()));
        assert_eq!(extension(".bashrc"), None);
        assert_eq!(extension("Makefile"), None);
        assert_eq!(extension("weird."), None);
    }

    #[test]
    fn kinds() {
        assert_eq!(file_kind("src", true), FileKind::Folder);
        assert_eq!(file_kind("cat.PNG", false), FileKind::Image);
        assert_eq!(file_kind("song.flac", false), FileKind::Audio);
        assert_eq!(file_kind("film.mkv", false), FileKind::Video);
        assert_eq!(file_kind("backup.tar.gz", false), FileKind::Archive);
        assert_eq!(file_kind("main.rs", false), FileKind::Code);
        assert_eq!(file_kind("cv.pdf", false), FileKind::Document);
        assert_eq!(file_kind("notes.md", false), FileKind::Text);
        assert_eq!(file_kind("setup.exe", false), FileKind::App);
        assert_eq!(file_kind("README", false), FileKind::Other);
    }

    #[test]
    fn a_folder_named_like_a_file_is_still_a_folder() {
        assert_eq!(file_kind("photos.png", true), FileKind::Folder);
    }
}
