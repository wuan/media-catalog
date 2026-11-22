use std::time::Duration;

/// A per-item failure surfaced in the run summary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemFailure {
    pub path: String,
    pub message: String,
}

/// One configured event and the number of items it matched.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventCount {
    pub slug: String,
    pub count: usize,
}

/// Counts and failures collected during a build.
#[derive(Debug, Clone, Default)]
pub struct RunSummary {
    pub dry_run: bool,
    pub events_only: bool,
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
    pub json_files_written: usize,
    pub errors: Vec<ItemFailure>,
    pub unsupported: Vec<String>,
    pub mount_warning: Option<String>,
    /// Per-event membership counts reported after publication.
    pub event_counts: Vec<EventCount>,
    /// Warnings for events beyond the supported curated size.
    pub event_warnings: Vec<String>,
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
        if self.events_only {
            lines.push(
                "Events-only run: scanning, probing, and thumbnail generation were skipped."
                    .to_string(),
            );
        }
        if !self.events_only {
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
        }
        if !self.dry_run {
            lines.push(format!("Output: {} JSON files", self.json_files_written));
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
        if !self.event_counts.is_empty() {
            lines.push(format!("Events: {}", self.event_counts.len()));
            for event in &self.event_counts {
                lines.push(format!("  - {}: {} items", event.slug, event.count));
            }
        }
        for warning in &self.event_warnings {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summary_reports_json_not_page_counts() {
        let summary = RunSummary {
            json_files_written: 3,
            thumbnails_generated: 2,
            deleted: 1,
            pruned: 4,
            ..RunSummary::default()
        };
        let rendered = summary.render();
        assert!(rendered.contains("Output: 3 JSON files"));
        assert!(rendered.contains("Removed state rows for 1 deleted items"));
        assert!(rendered.contains("Pruned 4 orphan thumbnails"));
        assert!(!rendered.to_ascii_lowercase().contains("pages"));
    }

    #[test]
    fn summary_reports_event_counts_and_large_event_warnings() {
        let summary = RunSummary {
            event_counts: vec![
                EventCount {
                    slug: "trip".into(),
                    count: 12,
                },
                EventCount {
                    slug: "season".into(),
                    count: 750,
                },
            ],
            event_warnings: vec![
                "event \"season\" matched 750 items (more than 500); consider splitting it".into(),
            ],
            ..RunSummary::default()
        };
        let rendered = summary.render();
        assert!(rendered.contains("Events: 2"));
        assert!(rendered.contains("- trip: 12 items"));
        assert!(rendered.contains("- season: 750 items"));
        assert!(rendered.contains("Warning: event \"season\" matched 750 items"));
    }

    #[test]
    fn summary_marks_events_only_runs() {
        let summary = RunSummary {
            events_only: true,
            json_files_written: 2,
            event_counts: vec![EventCount {
                slug: "may-trip".into(),
                count: 5,
            }],
            ..RunSummary::default()
        };
        let rendered = summary.render();
        assert!(rendered.contains("Events-only run"));
        assert!(rendered.contains("skipped"));
        assert!(!rendered.contains("Scanned:"));
        assert!(!rendered.contains("Probed:"));
        assert!(!rendered.contains("Thumbnails:"));
        assert!(rendered.contains("Output: 2 JSON files"));
        assert!(rendered.contains("- may-trip: 5 items"));
    }
}
