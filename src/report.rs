use std::time::Duration;

/// A per-item failure surfaced in the run summary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemFailure {
    pub path: String,
    pub message: String,
}

/// Counts and failures collected during a build.
#[derive(Debug, Clone, Default)]
pub struct RunSummary {
    pub dry_run: bool,
    pub scanned: usize,
    pub images: usize,
    pub videos: usize,
    pub new: usize,
    pub changed: usize,
    pub unchanged: usize,
    pub probed: usize,
    pub thumbnails_generated: usize,
    pub thumbnails_skipped: usize,
    pub thumbnails_failed: usize,
    pub thumbnails_unsupported: usize,
    pub deleted: usize,
    pub pruned: usize,
    pub pages_written: usize,
    pub json_files_written: usize,
    pub errors: Vec<ItemFailure>,
    pub unsupported: Vec<String>,
    pub mount_warning: Option<String>,
    pub blocked: bool,
    pub elapsed: Duration,
}

impl RunSummary {
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Human-readable summary. Error and unsupported lists are capped so a build
    /// with thousands of bad files stays readable.
    pub fn render(&self) -> String {
        let mut lines = Vec::new();
        if self.dry_run {
            lines.push("Dry run: no output or state was written.".to_string());
        }
        lines.push(format!(
            "Scanned: {} items ({} images, {} videos)",
            self.scanned, self.images, self.videos
        ));
        lines.push(format!(
            "New: {}, changed: {}, unchanged: {}",
            self.new, self.changed, self.unchanged
        ));
        lines.push(format!("Probed: {}", self.probed));
        lines.push(format!(
            "Thumbnails: {} generated, {} skipped, {} failed, {} unsupported",
            self.thumbnails_generated,
            self.thumbnails_skipped,
            self.thumbnails_failed,
            self.thumbnails_unsupported
        ));
        if !self.dry_run {
            lines.push(format!(
                "Output: {} pages, {} JSON files",
                self.pages_written, self.json_files_written
            ));
            if self.deleted > 0 {
                lines.push(format!(
                    "Removed state rows for {} deleted items",
                    self.deleted
                ));
            }
            if self.pruned > 0 {
                lines.push(format!("Pruned {} orphan thumbnails", self.pruned));
            }
        }
        if let Some(warning) = &self.mount_warning {
            lines.push(format!("Warning: {warning}"));
        }
        lines.push(format!("Errors: {}", self.errors.len()));
        for failure in self.errors.iter().take(25) {
            lines.push(format!("  - {}: {}", failure.path, failure.message));
        }
        if self.errors.len() > 25 {
            lines.push(format!("  ... and {} more", self.errors.len() - 25));
        }
        if self.thumbnails_unsupported > 0 {
            lines.push(format!(
                "Unsupported formats reported: {}",
                self.thumbnails_unsupported
            ));
            for path in self.unsupported.iter().take(10) {
                lines.push(format!("  - {path}"));
            }
            if self.unsupported.len() > 10 {
                lines.push(format!("  ... and {} more", self.unsupported.len() - 10));
            }
        }
        lines.push(format!("Elapsed: {:.1}s", self.elapsed.as_secs_f64()));
        lines.join("\n")
    }
}
