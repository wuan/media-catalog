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

/// Probe failures that share one root cause, with a few example paths.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeFailureGroup {
    pub cause: String,
    pub count: usize,
    pub samples: Vec<String>,
}

/// Maximum sample paths kept per probe failure cause.
const PROBE_FAILURE_SAMPLES: usize = 3;
/// Maximum probe failure causes listed in the rendered summary.
const PROBE_FAILURE_CAUSES_SHOWN: usize = 10;

/// Group `(path, cause)` pairs by cause, ordered by count (highest first) and
/// then by cause, keeping the first few paths of each group as samples.
pub fn group_probe_failures<I>(failures: I) -> Vec<ProbeFailureGroup>
where
    I: IntoIterator<Item = (String, String)>,
{
    let mut groups: Vec<ProbeFailureGroup> = Vec::new();
    let mut index: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for (path, cause) in failures {
        let slot = *index.entry(cause.clone()).or_insert_with(|| {
            groups.push(ProbeFailureGroup {
                cause,
                count: 0,
                samples: Vec::new(),
            });
            groups.len() - 1
        });
        let group = &mut groups[slot];
        group.count += 1;
        if group.samples.len() < PROBE_FAILURE_SAMPLES {
            group.samples.push(path);
        }
    }
    groups.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.cause.cmp(&b.cause)));
    groups
}

/// Warning for a systemic probe failure: at least 4 attempts and at least half
/// of them failed.
pub fn systemic_probe_warning(attempted: usize, failed: usize) -> Option<String> {
    if attempted < 4 || failed.saturating_mul(2) < attempted {
        return None;
    }
    Some(format!(
        "{failed} of {attempted} probes failed. The originals are probably unreadable in this \
         environment, so the published catalog will lack dimensions and camera metadata for \
         those items. Fix access to the media root and re-run to probe them again."
    ))
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
    /// Items the run tried to probe.
    pub probes_attempted: usize,
    /// Failed probes grouped by root cause, most frequent first.
    pub probe_failures: Vec<ProbeFailureGroup>,
    /// Set when most attempted probes failed.
    pub probe_warning: Option<String>,
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

    /// Total number of failed probes across all causes.
    pub fn probe_failure_count(&self) -> usize {
        self.probe_failures.iter().map(|group| group.count).sum()
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
                "Probe failures: {} of {}",
                self.probe_failure_count(),
                self.probes_attempted
            ));
            for group in self.probe_failures.iter().take(PROBE_FAILURE_CAUSES_SHOWN) {
                lines.push(format!(
                    "  - {}: {} (e.g. {})",
                    group.cause,
                    group.count,
                    group.samples.join(", ")
                ));
            }
            if self.probe_failures.len() > PROBE_FAILURE_CAUSES_SHOWN {
                lines.push(format!(
                    "  ... and {} more causes",
                    self.probe_failures.len() - PROBE_FAILURE_CAUSES_SHOWN
                ));
            }
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
        if let Some(warning) = &self.probe_warning {
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

    fn failures(cause: &str, count: usize) -> Vec<(String, String)> {
        (0..count)
            .map(|index| (format!("2026/11/{cause}-{index:04}.jpg"), cause.to_string()))
            .collect()
    }

    #[test]
    fn probe_failures_group_by_cause_with_bounded_samples() {
        let mut pairs = failures("decode", 2);
        pairs.extend(failures("Permission denied (os error 13)", 500));
        let groups = group_probe_failures(pairs);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].cause, "Permission denied (os error 13)");
        assert_eq!(groups[0].count, 500);
        assert_eq!(groups[0].samples.len(), 3);
        assert_eq!(groups[1].cause, "decode");
        assert_eq!(groups[1].count, 2);
        assert_eq!(groups[1].samples.len(), 2);
    }

    #[test]
    fn probe_failure_groups_tie_break_by_cause() {
        let mut pairs = failures("b", 1);
        pairs.extend(failures("a", 1));
        let groups = group_probe_failures(pairs);
        let causes: Vec<_> = groups.iter().map(|group| group.cause.as_str()).collect();
        assert_eq!(causes, ["a", "b"]);
    }

    #[test]
    fn systemic_probe_warning_needs_four_attempts_and_half_failed() {
        assert!(systemic_probe_warning(0, 0).is_none());
        assert!(systemic_probe_warning(3, 3).is_none());
        assert!(systemic_probe_warning(100, 2).is_none());
        assert!(systemic_probe_warning(5, 2).is_none());
        let warning = systemic_probe_warning(4, 2).expect("half failed");
        assert!(warning.contains("2 of 4 probes failed"));
        let warning = systemic_probe_warning(72_946, 72_946).expect("all failed");
        assert!(warning.contains("72946 of 72946 probes failed"));
        assert!(warning.contains("unreadable"));
        assert!(warning.contains("dimensions and camera metadata"));
    }

    #[test]
    fn summary_reports_zero_probe_failures_without_causes() {
        let summary = RunSummary {
            probed: 7,
            probes_attempted: 7,
            ..RunSummary::default()
        };
        let rendered = summary.render();
        let lines: Vec<_> = rendered.lines().collect();
        let probed = lines.iter().position(|line| *line == "Probed: 7").unwrap();
        assert_eq!(lines[probed + 1], "Probe failures: 0 of 7");
        assert!(lines[probed + 2].starts_with("Thumbnails:"));
        assert!(!rendered.contains("Warning:"));
    }

    #[test]
    fn summary_lists_probe_failures_by_cause() {
        let mut pairs = failures("Permission denied (os error 13)", 500);
        pairs.extend(failures("decode", 2));
        let summary = RunSummary {
            probes_attempted: 510,
            probed: 8,
            probe_failures: group_probe_failures(pairs),
            probe_warning: systemic_probe_warning(510, 502),
            ..RunSummary::default()
        };
        let rendered = summary.render();
        let lines: Vec<_> = rendered.lines().collect();
        let header = lines
            .iter()
            .position(|line| *line == "Probe failures: 502 of 510")
            .expect("probe failure line");
        assert_eq!(
            lines[header + 1],
            "  - Permission denied (os error 13): 500 (e.g. \
             2026/11/Permission denied (os error 13)-0000.jpg, \
             2026/11/Permission denied (os error 13)-0001.jpg, \
             2026/11/Permission denied (os error 13)-0002.jpg)"
        );
        assert_eq!(
            lines[header + 2],
            "  - decode: 2 (e.g. 2026/11/decode-0000.jpg, 2026/11/decode-0001.jpg)"
        );
        assert!(rendered.contains("Warning: 502 of 510 probes failed."));
    }

    #[test]
    fn summary_caps_probe_failure_causes() {
        let pairs: Vec<_> = (0..12)
            .flat_map(|index| failures(&format!("cause {index:02}"), 1))
            .collect();
        let summary = RunSummary {
            probes_attempted: 12,
            probe_failures: group_probe_failures(pairs),
            ..RunSummary::default()
        };
        let rendered = summary.render();
        assert!(rendered.contains("Probe failures: 12 of 12"));
        assert!(rendered.contains("  - cause 09: 1"));
        assert!(!rendered.contains("  - cause 10: 1"));
        assert!(rendered.contains("  ... and 2 more causes"));
    }

    #[test]
    fn events_only_summary_has_no_probe_lines() {
        let summary = RunSummary {
            events_only: true,
            ..RunSummary::default()
        };
        assert!(!summary.render().contains("Probe failures"));
    }
}
