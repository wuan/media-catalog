use std::fs::Metadata;
use std::time::{SystemTime, UNIX_EPOCH};

/// Fingerprint the source by size and modification time in nanoseconds.
pub fn fingerprint(size: u64, mtime_ns: i64) -> String {
    format!("{size}:{mtime_ns}")
}

/// URL-safe thumbnail version derived from the same fingerprint pair.
pub fn thumb_version(size: u64, mtime_ns: i64) -> String {
    format!("{:x}-{:x}", size, mtime_ns)
}

/// Read modification time as nanoseconds since the Unix epoch.
///
/// Values before the epoch are clamped to zero; this keeps the database column
/// within a signed 64-bit integer while remaining stable for the archive's use.
pub fn mtime_nanos(metadata: &Metadata) -> i64 {
    metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_nanos().min(i64::MAX as u128) as i64)
        .unwrap_or(0)
}

/// Current time as nanoseconds, used for run bookkeeping.
pub fn now_nanos() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos().min(i64::MAX as u128) as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fingerprint_is_stable_for_unchanged_input() {
        assert_eq!(fingerprint(1234, 99), fingerprint(1234, 99));
        assert_eq!(thumb_version(1234, 99), thumb_version(1234, 99));
    }

    #[test]
    fn fingerprint_changes_with_size_or_mtime() {
        assert_ne!(fingerprint(1234, 99), fingerprint(1235, 99));
        assert_ne!(fingerprint(1234, 99), fingerprint(1234, 100));
        assert_ne!(thumb_version(1234, 99), thumb_version(1235, 99));
    }

    #[test]
    fn version_is_url_safe() {
        let version = thumb_version(10, 15);
        assert_eq!(version, "a-f");
        assert!(version.chars().all(|c| c.is_ascii_hexdigit() || c == '-'));
    }
}
