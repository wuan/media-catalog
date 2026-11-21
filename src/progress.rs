use std::io::{IsTerminal, Write};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use indicatif::{ProgressBar, ProgressStyle};

/// Progress output for long-running phases.
///
/// Phase messages go to stderr unless `--quiet` is set, so they survive
/// redirection into cron or CI logs. On an interactive terminal, per-item work
/// also gets a live progress bar; otherwise the same work logs a counter at
/// roughly ten-percent intervals so redirected logs still show movement.
pub struct Progress {
    quiet: bool,
    interactive: bool,
}

impl Progress {
    pub fn new(quiet: bool) -> Self {
        Self {
            quiet,
            interactive: !quiet && std::io::stderr().is_terminal(),
        }
    }

    /// Announce the start or end of a pipeline phase.
    pub fn phase(&self, message: impl AsRef<str>) {
        if self.quiet {
            return;
        }
        eprintln!("==> {}", message.as_ref());
    }

    /// Reinforce a phase with secondary detail such as change counts.
    pub fn detail(&self, message: impl AsRef<str>) {
        if self.quiet {
            return;
        }
        eprintln!("    {}", message.as_ref());
    }

    /// Create a counter for a batch of work.
    pub fn bar(&self, total: usize, label: &'static str) -> StepBar {
        StepBar::new(total, label, self.quiet, self.interactive)
    }
}

/// A progress bar in a terminal, or periodic counter lines in non-interactive
/// logs. Safe to increment from rayon workers.
pub struct StepBar {
    label: &'static str,
    total: usize,
    quiet: bool,
    bar: Option<ProgressBar>,
    done: AtomicUsize,
    next_report: AtomicUsize,
    step: usize,
    started: Instant,
}

impl StepBar {
    fn new(total: usize, label: &'static str, quiet: bool, interactive: bool) -> Self {
        let bar = if !quiet && interactive && total > 0 {
            let bar = ProgressBar::new(total as u64);
            let style = ProgressStyle::with_template(
                "{spinner:.green} [{bar:32}] {pos}/{len} {msg} ({elapsed})",
            )
            .expect("valid progress template")
            .progress_chars("=>-");
            bar.set_style(style);
            bar.set_message(label);
            Some(bar)
        } else {
            None
        };

        // Small batches only need the caller's final line; larger batches report
        // at roughly ten-percent intervals without flooding the log.
        let step = if total < 100 {
            total.max(1)
        } else {
            (total / 10).max(50)
        };

        Self {
            label,
            total,
            quiet,
            bar,
            done: AtomicUsize::new(0),
            next_report: AtomicUsize::new(step),
            step,
            started: Instant::now(),
        }
    }

    /// Record completion of one item.
    pub fn inc(&self) {
        let done = self.done.fetch_add(1, Ordering::Relaxed) + 1;

        if let Some(bar) = &self.bar {
            bar.inc(1);
            return;
        }
        if self.quiet || done >= self.total {
            return;
        }

        let current = self.next_report.load(Ordering::Relaxed);
        if done >= current {
            let next = current.saturating_add(self.step);
            if self
                .next_report
                .compare_exchange(current, next, Ordering::Relaxed, Ordering::Relaxed)
                .is_ok()
            {
                let elapsed = self.started.elapsed().as_secs_f64();
                let mut stderr = std::io::stderr().lock();
                let _ = writeln!(
                    stderr,
                    "    {label}: {done}/{total} ({elapsed:.1}s)",
                    label = self.label,
                    total = self.total
                );
            }
        }
    }

    /// Clear the terminal bar, if present. Callers report the final counts.
    pub fn finish(&self) {
        if let Some(bar) = &self.bar {
            bar.finish_and_clear();
        }
    }
}
