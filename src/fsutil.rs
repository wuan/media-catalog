use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use anyhow::{Context, Result};

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Write bytes atomically through a temporary file in the destination directory,
/// followed by a rename. On failure the temporary file is removed so no partial
/// output is left behind.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("destination has no parent: {}", path.display()))?;
    fs::create_dir_all(parent)
        .with_context(|| format!("creating directory {}", parent.display()))?;

    let temp = create_temp_file(parent)?;
    let temp_path = temp.path.clone();
    let write_result = (|| -> io::Result<()> {
        let mut file = temp.file;
        file.write_all(bytes)?;
        file.flush()?;
        file.sync_all()?;
        fs::rename(&temp_path, path)?;
        Ok(())
    })();

    if let Err(error) = write_result {
        let _ = fs::remove_file(&temp_path);
        return Err(anyhow::Error::new(error).context(format!("writing {}", path.display())));
    }
    Ok(())
}

/// Atomically replace `path` with the contents produced by `write_fn`.
pub fn write_atomic_with<F>(path: &Path, write_fn: F) -> Result<()>
where
    F: FnOnce(&mut File) -> io::Result<()>,
{
    let parent = path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("destination has no parent: {}", path.display()))?;
    fs::create_dir_all(parent)
        .with_context(|| format!("creating directory {}", parent.display()))?;

    let temp = create_temp_file(parent)?;
    let temp_path = temp.path.clone();
    let write_result = (|| -> io::Result<()> {
        let mut file = temp.file;
        write_fn(&mut file)?;
        file.flush()?;
        file.sync_all()?;
        fs::rename(&temp_path, path)?;
        Ok(())
    })();

    if let Err(error) = write_result {
        let _ = fs::remove_file(&temp_path);
        return Err(anyhow::Error::new(error).context(format!("writing {}", path.display())));
    }
    Ok(())
}

struct TempFile {
    file: File,
    path: PathBuf,
}

fn create_temp_file(parent: &Path) -> Result<TempFile> {
    for _ in 0..100 {
        let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = parent.join(format!(
            ".media-catalog-tmp-{}-{}",
            std::process::id(),
            counter
        ));
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => {
                return Ok(TempFile { file, path });
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(anyhow::Error::new(error)
                    .context(format!("creating temporary file in {}", parent.display())));
            }
        }
    }
    anyhow::bail!(
        "could not create a unique temporary file in {}",
        parent.display()
    )
}

/// Create a unique temporary path (unopened) in `parent` with the given suffix.
pub fn temp_path(parent: &Path, suffix: &str) -> PathBuf {
    let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    parent.join(format!(
        ".media-catalog-tmp-{}-{}{}",
        std::process::id(),
        counter,
        suffix
    ))
}

/// Remove leftover temporary files in a directory created by this process family.
pub fn cleanup_temp_files(parent: &Path) {
    if let Ok(entries) = fs::read_dir(parent) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            if name.to_string_lossy().starts_with(".media-catalog-tmp-") {
                let _ = fs::remove_file(entry.path());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_and_replaces() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("a/b.txt");
        write_atomic(&path, b"one").unwrap();
        write_atomic(&path, b"two").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "two");
    }

    #[test]
    fn no_partial_file_after_failed_write() {
        let temp = tempfile::tempdir().unwrap();
        let destination = temp.path().join("target");
        // A directory at the destination makes the final rename fail.
        fs::create_dir(&destination).unwrap();
        let error = write_atomic(&destination, b"data").unwrap_err();
        assert!(error.to_string().contains("target"));
        let leftovers: Vec<_> = fs::read_dir(temp.path())
            .unwrap()
            .flatten()
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".media-catalog-tmp-")
            })
            .collect();
        assert!(leftovers.is_empty(), "temporary files left behind");
    }
}
