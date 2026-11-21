use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Serialize;
use walkdir::WalkDir;

use crate::fingerprint::{fingerprint, mtime_nanos};

/// Media kinds the catalog distinguishes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum MediaKind {
    Image,
    Video,
}

impl MediaKind {
    pub fn as_str(self) -> &'static str {
        match self {
            MediaKind::Image => "image",
            MediaKind::Video => "video",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "image" => Some(MediaKind::Image),
            "video" => Some(MediaKind::Video),
            _ => None,
        }
    }
}

/// A media file discovered during the scan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScannedFile {
    /// Slash-separated path relative to the media root.
    pub rel_path: String,
    pub year: i32,
    pub month: u32,
    pub kind: MediaKind,
    pub size: u64,
    pub mtime_ns: i64,
    pub fingerprint: String,
}

/// A per-file or per-directory problem found while scanning.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanError {
    pub path: String,
    pub message: String,
}

/// Result of scanning a media tree.
#[derive(Debug, Clone, Default)]
pub struct ScanResult {
    pub files: Vec<ScannedFile>,
    pub errors: Vec<ScanError>,
}

/// Extensions that are classified as images. HEIC/HEIF are deliberately included
/// so they are reported as unsupported by the v1 decoder rather than silently lost.
pub const IMAGE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "webp", "heic", "heif"];
/// Video extensions handled by the external ffmpeg/ffprobe tooling.
pub const VIDEO_EXTENSIONS: &[&str] = &["mp4", "mov", "m4v", "hevc", "mkv", "avi", "webm"];

/// Classify a file by its extension (case-insensitive).
pub fn classify(path: &Path) -> Option<MediaKind> {
    let extension = path.extension()?.to_str()?.to_ascii_lowercase();
    if IMAGE_EXTENSIONS.contains(&extension.as_str()) {
        Some(MediaKind::Image)
    } else if VIDEO_EXTENSIONS.contains(&extension.as_str()) {
        Some(MediaKind::Video)
    } else {
        None
    }
}

/// Image extensions that the v1 raster decoder can actually decode.
pub fn is_supported_image_extension(extension: &str) -> bool {
    matches!(
        extension.to_ascii_lowercase().as_str(),
        "jpg" | "jpeg" | "png" | "webp"
    )
}

/// Whether the v1 decoder boundary supports this file's extension.
pub fn is_decodable_extension(extension: &str) -> bool {
    let extension = extension.to_ascii_lowercase();
    is_supported_image_extension(&extension) || VIDEO_EXTENSIONS.contains(&extension.as_str())
}

/// Parse the leading `YYYY/MM` path segment. Returns `None` for files outside
/// the year/month hierarchy.
pub fn parse_year_month(rel_path: &Path) -> Option<(i32, u32)> {
    let mut components = rel_path.components();
    let year_part = components.next()?.as_os_str().to_str()?;
    let month_part = components.next()?.as_os_str().to_str()?;

    if year_part.len() != 4 || !year_part.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    if month_part.len() != 2 || !month_part.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }

    let year: i32 = year_part.parse().ok()?;
    let month: u32 = month_part.parse().ok()?;
    if !(1000..=2999).contains(&year) || !(1..=12).contains(&month) {
        return None;
    }
    Some((year, month))
}

/// Recursively scan `<root>/YYYY/MM/**` for supported media.
///
/// Hidden files and hidden directories are skipped entirely. Unsupported
/// extensions and files outside the year/month hierarchy are ignored. Files
/// that cannot be statted are collected in `errors` instead of aborting.
pub fn scan_media(root: &Path) -> Result<ScanResult> {
    let mut result = ScanResult::default();

    let walker = WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| {
            entry.depth() == 0
                || entry
                    .file_name()
                    .to_str()
                    .map(|name| !name.starts_with('.'))
                    .unwrap_or(false)
        });

    for entry in walker {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                let path = error
                    .path()
                    .map(|path| relative_string(root, path))
                    .unwrap_or_else(|| "<unknown>".to_string());
                result.errors.push(ScanError {
                    path,
                    message: error.to_string(),
                });
                continue;
            }
        };

        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path();
        let rel = match path.strip_prefix(root) {
            Ok(rel) => rel,
            Err(_) => continue,
        };

        let Some((year, month)) = parse_year_month(rel) else {
            continue;
        };
        let Some(kind) = classify(path) else {
            continue;
        };

        let metadata = match entry.metadata() {
            Ok(metadata) => metadata,
            Err(error) => {
                result.errors.push(ScanError {
                    path: relative_string(root, path),
                    message: error.to_string(),
                });
                continue;
            }
        };
        if !metadata.is_file() {
            continue;
        }

        let size = metadata.len();
        let mtime_ns = mtime_nanos(&metadata);
        result.files.push(ScannedFile {
            rel_path: relative_string(root, path),
            year,
            month,
            kind,
            size,
            mtime_ns,
            fingerprint: fingerprint(size, mtime_ns),
        });
    }

    // Walk order is not guaranteed; sort for deterministic downstream processing.
    result
        .files
        .sort_by(|left, right| left.rel_path.cmp(&right.rel_path));
    result
        .errors
        .sort_by(|left, right| left.path.cmp(&right.path));
    Ok(result)
}

/// Convert an absolute path under `root` into a slash-separated relative string.
pub fn relative_string(root: &Path, path: &Path) -> String {
    let rel = path.strip_prefix(root).unwrap_or(path);
    rel.components()
        .filter_map(|component| component.as_os_str().to_str())
        .collect::<Vec<_>>()
        .join("/")
}

/// Build a relative path from slash-separated components.
pub fn join_rel(parts: &[&str]) -> PathBuf {
    let mut path = PathBuf::new();
    for part in parts {
        path.push(part);
    }
    path
}

/// Metadata for an unreadable directory entry, used by callers that want a
/// friendly message.
pub fn scan_error(path: &Path, error: impl std::fmt::Display) -> ScanError {
    ScanError {
        path: path.display().to_string(),
        message: error.to_string(),
    }
}

/// Convenience wrapper used by tests and the pipeline.
pub fn scan(root: impl AsRef<Path>) -> Result<ScanResult> {
    scan_media(root.as_ref()).with_context(|| format!("scanning {}", root.as_ref().display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(path: &Path, contents: &[u8]) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, contents).unwrap();
    }

    #[test]
    fn parses_valid_year_month_paths() {
        assert_eq!(
            parse_year_month(Path::new("2024/05/a.jpg")),
            Some((2024, 5))
        );
        assert_eq!(
            parse_year_month(Path::new("1999/12/deep/b.jpg")),
            Some((1999, 12))
        );
        assert_eq!(parse_year_month(Path::new("2024/13/a.jpg")), None);
        assert_eq!(parse_year_month(Path::new("24/05/a.jpg")), None);
        assert_eq!(parse_year_month(Path::new("root.jpg")), None);
    }

    #[test]
    fn classifies_extensions_case_insensitively() {
        assert_eq!(classify(Path::new("a.JPG")), Some(MediaKind::Image));
        assert_eq!(classify(Path::new("a.mp4")), Some(MediaKind::Video));
        assert_eq!(classify(Path::new("a.heic")), Some(MediaKind::Image));
        assert_eq!(classify(Path::new("a.xmp")), None);
        assert!(!is_decodable_extension("heic"));
        assert!(is_decodable_extension("webp"));
    }

    #[test]
    fn scans_nested_month_folders_and_ignores_non_media() {
        let temp = tempfile::tempdir().unwrap();
        write(&temp.path().join("2024/05/a.jpg"), b"a");
        write(&temp.path().join("2024/05/nested/b.png"), b"b");
        write(&temp.path().join("2024/06/c.mp4"), b"c");
        write(&temp.path().join("2024/05/.hidden.jpg"), b"hidden");
        write(&temp.path().join("2024/05/.hidden-dir/x.jpg"), b"hidden");
        write(&temp.path().join("2024/05/notes.txt"), b"text");
        write(&temp.path().join("loose.jpg"), b"loose");
        write(&temp.path().join("2024/05/sidecar.xmp"), b"xmp");

        let result = scan_media(temp.path()).unwrap();
        let paths: Vec<_> = result
            .files
            .iter()
            .map(|file| file.rel_path.as_str())
            .collect();
        assert_eq!(
            paths,
            vec!["2024/05/a.jpg", "2024/05/nested/b.png", "2024/06/c.mp4"]
        );
        assert_eq!(result.files[1].year, 2024);
        assert_eq!(result.files[1].month, 5);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn groups_by_folder_not_filename() {
        let temp = tempfile::tempdir().unwrap();
        write(&temp.path().join("2023/12/photo-from-2024.jpg"), b"x");
        let result = scan_media(temp.path()).unwrap();
        assert_eq!(result.files[0].year, 2023);
        assert_eq!(result.files[0].month, 12);
    }

    #[test]
    fn reports_unreadable_entries() {
        let temp = tempfile::tempdir().unwrap();
        write(&temp.path().join("2024/05/a.jpg"), b"a");
        let missing = temp.path().join("2024/05/does-not-exist.jpg");
        let error = scan_error(&missing, "permission denied");
        assert!(error.message.contains("permission denied"));
        assert!(error.path.contains("does-not-exist"));
    }
}
