use std::fs::{self, File};
use std::path::{Path, PathBuf};

use fs2::FileExt;
use thiserror::Error;

/// Errors acquiring the build lock.
#[derive(Debug, Error)]
pub enum LockError {
    #[error("another build is already running against this state database (lock: {0})")]
    Busy(PathBuf),
    #[error("could not open build lock {path}: {source}")]
    Open {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

/// Held for the duration of a build. Dropping it releases the OS lock; the lock
/// file itself is intentionally left in place so a racing process cannot lock a
/// deleted inode while another process locks a freshly created one.
#[derive(Debug)]
pub struct BuildLock {
    file: File,
    path: PathBuf,
}

impl Drop for BuildLock {
    fn drop(&mut self) {
        // Closing the descriptor releases the lock on most platforms; unlocking
        // explicitly makes the guarantee independent of close semantics.
        let _ = self.file.unlock();
    }
}

impl BuildLock {
    /// Acquire an exclusive lock beside the state database.
    pub fn acquire(state_db: &Path) -> Result<Self, LockError> {
        let parent = state_db.parent().unwrap_or_else(|| Path::new("."));
        fs::create_dir_all(parent).map_err(|source| LockError::Open {
            path: parent.to_path_buf(),
            source,
        })?;
        let path = parent.join("build.lock");
        let file = File::options()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path)
            .map_err(|source| LockError::Open {
                path: path.clone(),
                source,
            })?;
        match file.try_lock_exclusive() {
            Ok(()) => Ok(Self { file, path }),
            Err(source) if source.kind() == std::io::ErrorKind::WouldBlock => {
                Err(LockError::Busy(path))
            }
            Err(source) => Err(LockError::Open { path, source }),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn second_holder_is_rejected_and_drop_releases() {
        let temp = tempfile::tempdir().unwrap();
        let state = temp.path().join("state.sqlite");
        let lock = BuildLock::acquire(&state).unwrap();
        let error = BuildLock::acquire(&state).unwrap_err();
        assert!(matches!(error, LockError::Busy(_)));
        drop(lock);
        let reacquired = BuildLock::acquire(&state).unwrap();
        drop(reacquired);
    }

    #[test]
    fn released_after_failure() {
        let temp = tempfile::tempdir().unwrap();
        let state = temp.path().join("state.sqlite");
        let result: Result<(), &str> = {
            let _lock = BuildLock::acquire(&state).unwrap();
            Err("simulated failure")
        };
        assert!(result.is_err());
        assert!(BuildLock::acquire(&state).is_ok());
    }
}
