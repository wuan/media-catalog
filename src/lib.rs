pub mod config;
pub mod events;
pub mod fingerprint;
pub mod fsutil;
pub mod json;
pub mod lock;
pub mod model;
pub mod paths;
pub mod probe;
pub mod progress;
pub mod prune;
pub mod render;
pub mod report;
pub mod scan;
pub mod server;
pub mod state;
pub mod thumbs;

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::time::Instant;

use anyhow::{Context, Result};
use rayon::prelude::*;
use rayon::ThreadPoolBuilder;

use crate::config::{Cli, Config};
use crate::lock::BuildLock;
use crate::model::CatalogItemInput;
use crate::paths::thumb_rel_path;
use crate::probe::ProbeResult;
use crate::progress::Progress;
use crate::report::{EventCount, ItemFailure, RunSummary};
use crate::scan::{is_decodable_extension, MediaKind, ScannedFile};
use crate::state::{StateDb, ThumbStatus};
use crate::thumbs::{ThumbError, ThumbInput};

/// Result of a build run, including the summary and whether a safety guard blocked cleanup.
#[derive(Debug, Clone)]
pub struct RunOutcome {
    pub summary: RunSummary,
    pub blocked: bool,
}

/// Resolve configuration from CLI/TOML and run the pipeline.
pub fn run(cli: Cli) -> Result<RunOutcome> {
    let config = Config::load(cli)?;
    run_with_config(config)
}

struct ProbeWork {
    file: ScannedFile,
    kind: MediaKind,
}

struct ProbeOutcome {
    rel_path: String,
    result: Result<ProbeResult>,
    fallback_captured_at: String,
}

struct ThumbWork {
    rel_path: String,
    source: PathBuf,
    destination: PathBuf,
    extension: String,
    version: String,
    duration_ms: Option<u64>,
}

struct ThumbOutcome {
    work: ThumbWork,
    result: Result<(), ThumbError>,
}

/// Run the full pipeline with a resolved configuration.
///
/// The order is: validate tools -> lock -> open state -> scan -> diff ->
/// probe changed -> thumbnail changed -> publish JSON -> cleanup -> report.
pub fn run_with_config(config: Config) -> Result<RunOutcome> {
    config.validate()?;
    let event_definitions = events::load_events(config.events_file.as_deref())?;
    if !config.events_only {
        probe::ensure_external_tools().context("configuration error")?;
    }
    let progress = Progress::new(config.quiet);
    progress.detail(format!("Data root: {}", config.data_root.display()));
    progress.detail(format!("State root: {}", config.state_root.display()));
    if let Some(path) = &config.events_file {
        progress.detail(format!("Events: {}", path.display()));
    }

    let started = Instant::now();
    let mut summary = RunSummary {
        dry_run: config.dry_run,
        events_only: config.events_only,
        ..RunSummary::default()
    };

    // A dry run neither locks nor writes; it may read an existing state database.
    let _lock: Option<BuildLock> = if config.dry_run {
        None
    } else {
        Some(BuildLock::acquire(&config.state_db)?)
    };

    // Events-only mode requires an existing state database; fail before opening
    // (which would create an empty one) and before writing any output.
    if config.events_only && !config.dry_run && !config.state_db.exists() {
        anyhow::bail!(
            "events-only mode requires an existing state database; run a full build first"
        );
    }

    let mut state = if config.dry_run {
        if config.state_db.exists() {
            Some(StateDb::open_read_only(&config.state_db)?)
        } else {
            None
        }
    } else {
        Some(StateDb::open(&config.state_db)?)
    };

    let previous_items = match &state {
        Some(database) => database.load_items()?,
        None => HashMap::new(),
    };
    let previous_count = previous_items.len();

    let started_at = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let run_id = match &mut state {
        Some(database) if !config.dry_run => database.start_run(&started_at)?,
        _ => 0,
    };

    if config.events_only {
        return run_events_only(
            &config,
            &mut state,
            run_id,
            &event_definitions,
            &progress,
            started,
        );
    }

    progress.phase(format!("Scanning {}", config.media_root.display()));
    let scan_result = scan::scan(&config.media_root).with_context(|| "scanning the media root")?;
    summary.scanned = scan_result.files.len();
    summary.images = scan_result
        .files
        .iter()
        .filter(|file| file.kind == MediaKind::Image)
        .count();
    summary.videos = summary.scanned - summary.images;
    for error in &scan_result.errors {
        push_error(&mut summary, &error.path, &error.message);
    }
    progress.phase(format!(
        "Scanned {} items ({} images, {} videos, {} errors)",
        summary.scanned,
        summary.images,
        summary.videos,
        scan_result.errors.len()
    ));

    let scanned_count = summary.scanned;
    detect_mount_drop(&config, &mut summary, previous_count, scanned_count);
    if let Some(warning) = &summary.mount_warning {
        progress.phase(format!("Warning: {warning}"));
    }

    let mut new_paths = HashSet::new();
    let mut changed_paths = HashSet::new();
    for file in &scan_result.files {
        match previous_items.get(&file.rel_path) {
            None => {
                summary.new += 1;
                new_paths.insert(file.rel_path.clone());
            }
            Some(record) if record.fingerprint != file.fingerprint => {
                summary.changed += 1;
                changed_paths.insert(file.rel_path.clone());
            }
            Some(_) => summary.unchanged += 1,
        }
    }
    progress.detail(format!(
        "New: {}, changed: {}, unchanged: {}",
        summary.new, summary.changed, summary.unchanged
    ));

    if config.dry_run {
        summary.thumbnails_unsupported = scan_result
            .files
            .iter()
            .filter(|file| !is_decodable_extension(&extension(&file.rel_path)))
            .count();
        summary.elapsed = started.elapsed();
        progress.phase("Dry run complete: no output or state was written.");
        let blocked = summary.blocked;
        return Ok(RunOutcome { summary, blocked });
    }

    let database = state
        .as_mut()
        .expect("non-dry runs always open the state database");
    database
        .upsert_scanned(&scan_result.files, run_id)
        .context("updating scan state")?;

    let records_after_scan = database.load_items()?;

    // Unsupported formats are cataloged, reported, and given a placeholder tile.
    for file in &scan_result.files {
        if is_decodable_extension(&extension(&file.rel_path)) {
            continue;
        }
        let record = records_after_scan.get(&file.rel_path);
        let fallback = model::capture_timestamp(
            record.and_then(|record| record.captured_at.clone()),
            file.mtime_ns,
        );
        database.set_thumb(&file.rel_path, None, None, ThumbStatus::Unsupported, None)?;
        database.set_probe(&file.rel_path, None, None, None, Some(&fallback), None)?;
        summary.thumbnails_unsupported += 1;
        summary.unsupported.push(file.rel_path.clone());
    }

    let thread_pool = ThreadPoolBuilder::new()
        .num_threads(config.jobs)
        .build()
        .context("creating thumbnail worker pool")?;

    // Probe only new/changed items and unchanged rows missing cached metadata.
    let probe_work: Vec<ProbeWork> = scan_result
        .files
        .iter()
        .filter(|file| is_decodable_extension(&extension(&file.rel_path)))
        .filter(|file| {
            let record = records_after_scan.get(&file.rel_path);
            new_paths.contains(&file.rel_path)
                || changed_paths.contains(&file.rel_path)
                || record
                    .map(|record| record.width.is_none() || record.captured_at.is_none())
                    .unwrap_or(true)
        })
        .map(|file| ProbeWork {
            file: file.clone(),
            kind: file.kind,
        })
        .collect();

    if probe_work.is_empty() {
        progress.phase("Probing: nothing to do");
    } else {
        progress.phase(format!("Probing {} items", probe_work.len()));
    }
    let probe_progress = progress.bar(probe_work.len(), "probed");

    let probe_results: Vec<ProbeOutcome> = thread_pool.install(|| {
        probe_work
            .par_iter()
            .map(|work| {
                let source = config.media_root.join(&work.file.rel_path);
                let result = match work.kind {
                    MediaKind::Image => probe::probe_image(&source, work.file.mtime_ns),
                    MediaKind::Video => probe::probe_video(&source, work.file.mtime_ns),
                };
                let outcome = ProbeOutcome {
                    rel_path: work.file.rel_path.clone(),
                    result,
                    fallback_captured_at: probe::timestamp_from_nanos(work.file.mtime_ns),
                };
                probe_progress.inc();
                outcome
            })
            .collect()
    });
    probe_progress.finish();

    let errors_before_probe = summary.errors.len();
    for outcome in probe_results {
        match outcome.result {
            Ok(result) => {
                database.set_probe(
                    &outcome.rel_path,
                    result.width,
                    result.height,
                    result.duration_ms,
                    result.captured_at.as_deref(),
                    result.model.as_deref(),
                )?;
                summary.probed += 1;
            }
            Err(error) => {
                database.set_probe(
                    &outcome.rel_path,
                    None,
                    None,
                    None,
                    Some(&outcome.fallback_captured_at),
                    None,
                )?;
                push_error(&mut summary, &outcome.rel_path, &format!("{error:#}"));
            }
        }
    }

    let probe_failures = summary.errors.len().saturating_sub(errors_before_probe);
    progress.phase(format!(
        "Probed {} items ({} failed)",
        summary.probed, probe_failures
    ));

    let records_after_probe = database.load_items()?;

    // Decide which thumbnails need work.
    let mut thumb_work = Vec::new();
    for file in &scan_result.files {
        let extension = extension(&file.rel_path);
        if !thumbs::format_supported(&extension) {
            continue;
        }
        let record = records_after_probe.get(&file.rel_path);
        let version = fingerprint::thumb_version(file.size, file.mtime_ns);
        let thumb_rel = thumb_rel_path(&file.rel_path);
        let destination = config.data_root.join(&thumb_rel);
        let needs_work = config.force
            || record
                .map(|record| {
                    record.thumb_status != ThumbStatus::Ok
                        || record.thumb_path.as_deref() != Some(thumb_rel.as_str())
                        || record.thumb_version.as_deref() != Some(version.as_str())
                })
                .unwrap_or(true)
            || !destination.exists();

        if needs_work {
            thumb_work.push(ThumbWork {
                rel_path: file.rel_path.clone(),
                source: config.media_root.join(&file.rel_path),
                destination,
                extension,
                version,
                duration_ms: record.and_then(|record| record.duration_ms),
            });
        } else {
            summary.thumbnails_skipped += 1;
        }
    }

    if thumb_work.is_empty() {
        progress.phase(format!(
            "Thumbnails: nothing to do ({} already up to date)",
            summary.thumbnails_skipped
        ));
    } else {
        progress.phase(format!(
            "Generating {} thumbnails ({} already up to date)",
            thumb_work.len(),
            summary.thumbnails_skipped
        ));
    }
    let thumb_progress = progress.bar(thumb_work.len(), "thumbnails");

    for chunk in thumb_work.chunks(config.jobs.max(1)) {
        let outcomes: Vec<ThumbOutcome> = thread_pool.install(|| {
            chunk
                .par_iter()
                .map(|work| {
                    let result = thumbs::generate_thumbnail(
                        &work.source,
                        &work.destination,
                        &work.extension,
                        &ThumbInput {
                            duration_ms: work.duration_ms,
                        },
                    );
                    ThumbOutcome {
                        work: ThumbWork {
                            rel_path: work.rel_path.clone(),
                            source: work.source.clone(),
                            destination: work.destination.clone(),
                            extension: work.extension.clone(),
                            version: work.version.clone(),
                            duration_ms: work.duration_ms,
                        },
                        result,
                    }
                })
                .collect()
        });

        for outcome in outcomes {
            match outcome.result {
                Ok(()) => {
                    database.set_thumb(
                        &outcome.work.rel_path,
                        Some(&thumb_rel_path(&outcome.work.rel_path)),
                        Some(&outcome.work.version),
                        ThumbStatus::Ok,
                        None,
                    )?;
                    summary.thumbnails_generated += 1;
                }
                Err(ThumbError::Unsupported(_format)) => {
                    database.set_thumb(
                        &outcome.work.rel_path,
                        None,
                        None,
                        ThumbStatus::Unsupported,
                        None,
                    )?;
                    summary.thumbnails_unsupported += 1;
                    summary.unsupported.push(outcome.work.rel_path.clone());
                }
                Err(ThumbError::Failed(error)) => {
                    let message = format!("{error:#}");
                    database.set_thumb(
                        &outcome.work.rel_path,
                        None,
                        None,
                        ThumbStatus::Failed,
                        Some(&message),
                    )?;
                    summary.thumbnails_failed += 1;
                    push_error(&mut summary, &outcome.work.rel_path, &message);
                }
            }
            thumb_progress.inc();
        }
    }
    thumb_progress.finish();
    progress.phase(format!(
        "Thumbnails: {} generated, {} skipped, {} failed, {} unsupported",
        summary.thumbnails_generated,
        summary.thumbnails_skipped,
        summary.thumbnails_failed,
        summary.thumbnails_unsupported
    ));

    let records = database.load_items()?;
    let mut inputs = Vec::with_capacity(scan_result.files.len());
    for file in &scan_result.files {
        let record = records.get(&file.rel_path);
        let thumb_available = record
            .map(|record| record.thumb_status == ThumbStatus::Ok)
            .unwrap_or(false);
        inputs.push(CatalogItemInput {
            rel_path: file.rel_path.clone(),
            year: file.year,
            month: file.month,
            kind: file.kind,
            captured_at: record.and_then(|record| record.captured_at.clone()),
            model: record.and_then(|record| record.model.clone()),
            mtime_ns: file.mtime_ns,
            size: file.size,
            width: record.and_then(|record| record.width),
            height: record.and_then(|record| record.height),
            duration_ms: record.and_then(|record| record.duration_ms),
            thumb_version: record
                .and_then(|record| record.thumb_version.clone())
                .filter(|_| thumb_available),
            thumb_available,
        });
    }

    let mut catalog =
        model::build_catalog(inputs, &config.media_url_prefix, &config.thumb_url_prefix);
    events::attach_events(&mut catalog, event_definitions)?;

    progress.phase("Publishing JSON catalog");
    summary.json_files_written = json::write_json(&config.data_root, &catalog)?;
    progress.phase(format!(
        "Published {} JSON files",
        summary.json_files_written
    ));

    summary.event_counts = catalog
        .events
        .iter()
        .map(|event| EventCount {
            slug: event.slug.clone(),
            count: event.item_count,
        })
        .collect();
    for event in &catalog.events {
        if let Some(warning) = events::large_event_warning(event) {
            progress.phase(format!("Warning: {warning}"));
            summary.event_warnings.push(warning);
        }
    }
    if !catalog.events.is_empty() {
        progress.phase(format!("Events: {} configured", catalog.events.len()));
    }

    if summary.blocked {
        summary.deleted = 0;
        progress.phase("Cleanup skipped: mount-drop guard is active");
    } else {
        summary.deleted = database.delete_unseen(run_id)?;
        progress.phase(format!(
            "Removed {} state rows for deleted items",
            summary.deleted
        ));
    }

    if config.prune {
        if summary.blocked {
            // The mount guard requires an explicit override before destructive cleanup.
            progress.phase("Prune skipped: mount-drop guard is active");
        } else {
            progress.phase("Pruning orphan thumbnails");
            let mut expected = HashSet::new();
            for file in &scan_result.files {
                if let Some(record) = records.get(&file.rel_path) {
                    if record.thumb_status == ThumbStatus::Ok {
                        expected.insert(thumb_rel_path(&file.rel_path));
                    }
                }
            }
            summary.pruned = prune::prune_orphans(&config.data_root, &expected)?;
            progress.phase(format!("Pruned {} orphan thumbnails", summary.pruned));
        }
    }

    summary.elapsed = started.elapsed();
    let status = if summary.blocked {
        "blocked"
    } else if summary.has_errors() {
        "failed"
    } else {
        "ok"
    };
    database.finish_run(
        run_id,
        status,
        summary.scanned,
        summary.errors.len(),
        &chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
    )?;

    let blocked = summary.blocked;
    Ok(RunOutcome { summary, blocked })
}

/// Map a persisted item record to a catalog input row.
fn record_to_input(record: &state::ItemRecord) -> CatalogItemInput {
    let thumb_available = record.thumb_status == ThumbStatus::Ok;
    CatalogItemInput {
        rel_path: record.path.clone(),
        year: record.year,
        month: record.month,
        kind: record.kind,
        captured_at: record.captured_at.clone(),
        model: record.model.clone(),
        mtime_ns: record.mtime,
        size: record.size,
        width: record.width,
        height: record.height,
        duration_ms: record.duration_ms,
        thumb_version: record.thumb_version.clone().filter(|_| thumb_available),
        thumb_available,
    }
}

/// Events-only fast path: rebuild the catalog from persisted state and republish JSON.
fn run_events_only(
    config: &Config,
    state: &mut Option<StateDb>,
    run_id: i64,
    event_definitions: &[model::EventMeta],
    progress: &Progress,
    started: Instant,
) -> Result<RunOutcome> {
    let database = match state {
        Some(db) => db,
        None => {
            anyhow::bail!(
                "events-only mode requires an existing state database; run a full build first"
            );
        }
    };

    let items = database.load_items()?;
    let mut inputs: Vec<CatalogItemInput> = items.values().map(record_to_input).collect();
    inputs.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));

    let mut catalog =
        model::build_catalog(inputs, &config.media_url_prefix, &config.thumb_url_prefix);
    events::attach_events(&mut catalog, event_definitions.to_vec())?;

    let mut summary = RunSummary {
        dry_run: config.dry_run,
        events_only: true,
        ..RunSummary::default()
    };

    if config.dry_run {
        summary.event_counts = catalog
            .events
            .iter()
            .map(|event| EventCount {
                slug: event.slug.clone(),
                count: event.item_count,
            })
            .collect();
        summary.elapsed = started.elapsed();
        progress.phase("Dry run complete: no output or state was written.");
        return Ok(RunOutcome {
            summary,
            blocked: false,
        });
    }

    progress.phase("Publishing JSON catalog (events-only)");
    summary.json_files_written = json::write_json(&config.data_root, &catalog)?;
    progress.phase(format!(
        "Published {} JSON files",
        summary.json_files_written
    ));

    summary.event_counts = catalog
        .events
        .iter()
        .map(|event| EventCount {
            slug: event.slug.clone(),
            count: event.item_count,
        })
        .collect();
    for event in &catalog.events {
        if let Some(warning) = events::large_event_warning(event) {
            progress.phase(format!("Warning: {warning}"));
            summary.event_warnings.push(warning);
        }
    }

    summary.elapsed = started.elapsed();
    let item_count = catalog.item_count;
    database.finish_run(
        run_id,
        "events-only",
        item_count,
        0,
        &chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
    )?;

    Ok(RunOutcome {
        summary,
        blocked: false,
    })
}

fn extension(rel_path: &str) -> String {
    std::path::Path::new(rel_path)
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
}

fn push_error(summary: &mut RunSummary, path: &str, message: &str) {
    if !summary.errors.iter().any(|failure| failure.path == path) {
        summary.errors.push(ItemFailure {
            path: path.to_string(),
            message: message.to_string(),
        });
    }
}

fn detect_mount_drop(config: &Config, summary: &mut RunSummary, previous: usize, current: usize) {
    let dropped = previous >= 4 && current.saturating_mul(2) < previous;
    if !dropped {
        return;
    }
    summary.mount_warning = Some(format!(
        "item count dropped from {previous} to {current}, which may indicate a stale or missing NAS mount. \
         Skipping prune and state cleanup until deletions are confirmed."
    ));
    if config.allow_mount_drop {
        summary.mount_warning = Some(format!(
            "item count dropped from {previous} to {current}; confirmed with --allow-mount-drop."
        ));
    } else {
        summary.blocked = true;
    }
}
