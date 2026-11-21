use std::collections::HashSet;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use walkdir::WalkDir;

/// Delete files under `public/thumbs/` that are not in `expected`.
///
/// `expected` contains public-tree-relative paths such as
/// `thumbs/2024/05/a.jpg.webp`. The walk is confined to the thumbnail tree;
/// symlinks are never followed or removed and empty directories are cleaned up.
pub fn prune_orphans(public_root: &Path, expected: &HashSet<String>) -> Result<usize> {
    let thumbs_root = public_root.join("thumbs");
    if !thumbs_root.exists() {
        return Ok(0);
    }
    if !thumbs_root.is_dir() {
        bail!(
            "thumbnail tree is not a directory: {}",
            thumbs_root.display()
        );
    }
    let canonical_root =
        std::fs::canonicalize(&thumbs_root).context("resolving the thumbnail tree")?;

    let mut removed = 0usize;
    let mut directories: Vec<PathBuf> = Vec::new();

    for entry in WalkDir::new(&thumbs_root).follow_links(false) {
        let entry = entry.with_context(|| format!("walking {}", thumbs_root.display()))?;
        let path = entry.path();

        // Defense in depth: never act on a path that resolves outside thumbs/.
        if entry.file_type().is_symlink() {
            continue;
        }
        if !path.starts_with(&canonical_root) && !path.starts_with(&thumbs_root) {
            continue;
        }

        if entry.file_type().is_dir() {
            if path != thumbs_root {
                directories.push(path.to_path_buf());
            }
            continue;
        }
        if !entry.file_type().is_file() {
            continue;
        }

        let rel = path
            .strip_prefix(public_root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");
        if !expected.contains(&rel) {
            std::fs::remove_file(path)
                .with_context(|| format!("removing orphan thumbnail {}", path.display()))?;
            removed += 1;
        }
    }

    // Deepest directories first so emptied parents can also be removed.
    directories.sort_by_key(|path| std::cmp::Reverse(path.components().count()));
    for directory in directories {
        if directory.starts_with(&thumbs_root) {
            let _ = std::fs::remove_dir(&directory);
        }
    }
    Ok(removed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn touch(path: &Path) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, b"x").unwrap();
    }

    #[test]
    fn removes_only_orphans_inside_the_thumb_tree() {
        let temp = tempfile::tempdir().unwrap();
        touch(&temp.path().join("thumbs/2024/05/keep.jpg.webp"));
        touch(&temp.path().join("thumbs/2024/05/orphan.jpg.webp"));
        touch(&temp.path().join("thumbs/2019/01/old.jpg.webp"));
        touch(&temp.path().join("index.html"));
        touch(&temp.path().join("catalog/index.json"));

        let expected = HashSet::from(["thumbs/2024/05/keep.jpg.webp".to_string()]);
        let removed = prune_orphans(temp.path(), &expected).unwrap();
        assert_eq!(removed, 2);
        assert!(temp.path().join("thumbs/2024/05/keep.jpg.webp").exists());
        assert!(!temp.path().join("thumbs/2024/05/orphan.jpg.webp").exists());
        assert!(!temp.path().join("thumbs/2019").exists());
        assert!(temp.path().join("index.html").exists());
        assert!(temp.path().join("catalog/index.json").exists());
    }

    #[test]
    fn does_nothing_when_tree_is_absent() {
        let temp = tempfile::tempdir().unwrap();
        assert_eq!(prune_orphans(temp.path(), &HashSet::new()).unwrap(), 0);
    }

    #[test]
    fn refuses_a_file_in_place_of_the_thumb_tree() {
        let temp = tempfile::tempdir().unwrap();
        std::fs::write(temp.path().join("thumbs"), b"x").unwrap();
        let error = prune_orphans(temp.path(), &HashSet::new()).unwrap_err();
        assert!(error.to_string().contains("not a directory"));
    }
}
