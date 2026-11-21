use std::path::{Path, PathBuf};

use percent_encoding::{utf8_percent_encode, AsciiSet, CONTROLS};

/// Characters escaped inside a single URL path segment. Non-ASCII characters
/// are percent-encoded automatically by `utf8_percent_encode`.
const SEGMENT_ENCODE_SET: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b'#')
    .add(b'%')
    .add(b'<')
    .add(b'>')
    .add(b'?')
    .add(b'`')
    .add(b'{')
    .add(b'}')
    .add(b'^')
    .add(b'|')
    .add(b'\\');

/// Percent-encode a slash-separated relative path segment by segment.
pub fn encode_path(rel_path: &str) -> String {
    rel_path
        .split('/')
        .filter(|segment| !segment.is_empty())
        .map(|segment| utf8_percent_encode(segment, SEGMENT_ENCODE_SET).to_string())
        .collect::<Vec<_>>()
        .join("/")
}

/// Build a root-relative URL for a media or thumbnail path.
pub fn url_for(prefix: &str, rel_path: &str) -> String {
    format!("{}/{}", prefix.trim_end_matches('/'), encode_path(rel_path))
}

/// Build the thumbnail URL with a version query parameter.
pub fn thumb_url(prefix: &str, src_rel_path: &str, version: &str) -> String {
    format!(
        "{}.webp?v={}",
        url_for(prefix, src_rel_path),
        utf8_percent_encode(version, SEGMENT_ENCODE_SET)
    )
}

/// Public-tree-relative thumbnail path, mirroring the source path and appending
/// `.webp` to the full file name (rather than replacing the extension).
pub fn thumb_rel_path(src_rel_path: &str) -> String {
    format!("thumbs/{src_rel_path}.webp")
}

/// Absolute thumbnail destination under the public root.
pub fn thumb_abs_path(public_root: &Path, src_rel_path: &str) -> PathBuf {
    public_root.join(thumb_rel_path(src_rel_path))
}

/// Public-tree-relative path for a year page.
pub fn year_page_rel(year: i32) -> String {
    format!("{year}/index.html")
}

/// Public-tree-relative path for a month page.
pub fn month_page_rel(year: i32, month: u32) -> String {
    format!("{year}/{month:02}/index.html")
}

/// Root-relative navigation URL for a year page.
pub fn year_page_url(year: i32) -> String {
    format!("/{year}/")
}

/// Root-relative navigation URL for a month page.
pub fn month_page_url(year: i32, month: u32) -> String {
    format!("/{year}/{month:02}/")
}

/// Stem/name of an item for display.
pub fn item_name(rel_path: &str) -> String {
    rel_path.rsplit('/').next().unwrap_or(rel_path).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_spaces_and_unicode() {
        assert_eq!(
            url_for("/media", "2024/05/Familie Müller #1.jpg"),
            "/media/2024/05/Familie%20M%C3%BCller%20%231.jpg"
        );
    }

    #[test]
    fn encodes_percent_and_hash() {
        assert_eq!(encode_path("50%.jpg"), "50%25.jpg");
        assert_eq!(encode_path("a#b.jpg"), "a%23b.jpg");
    }

    #[test]
    fn leaves_nested_folders_separate() {
        assert_eq!(
            url_for("/thumbs", "2024/05/event/one two.jpg"),
            "/thumbs/2024/05/event/one%20two.jpg"
        );
    }

    #[test]
    fn thumb_paths_mirror_and_append() {
        assert_eq!(thumb_rel_path("2024/05/a.jpg"), "thumbs/2024/05/a.jpg.webp");
        assert_eq!(
            thumb_url("/thumbs", "2024/05/a.jpg", "a-f"),
            "/thumbs/2024/05/a.jpg.webp?v=a-f"
        );
    }

    #[test]
    fn same_basenames_in_different_folders_get_unique_paths() {
        assert_ne!(
            thumb_rel_path("2024/05/a.jpg"),
            thumb_rel_path("2024/06/a.jpg")
        );
        assert_ne!(
            thumb_rel_path("2024/05/a.jpg"),
            thumb_rel_path("2024/05/a.png")
        );
    }

    #[test]
    fn prefixes_with_trailing_slash_do_not_double_up() {
        assert_eq!(url_for("/media/", "2024/a.jpg"), "/media/2024/a.jpg");
    }
}
