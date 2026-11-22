# Design

## Context

See proposal.md (Why) for motivation. A full generator run
(`run_with_config` in `src/lib.rs`) is ordered: tool check -> lock -> state
open -> scan -> diff -> probe -> thumbnails -> build catalog -> attach events
-> publish JSON -> cleanup. On a large archive the scan/probe/thumbnail phases
dominate, so iterating on `events.toml` is slow even when nothing on disk
changed. Two existing properties make a fast path possible: the state database
(`ItemRecord` in `src/state.rs`) already stores every field needed to rebuild
the published catalog (path, year/month, kind, fingerprint, dimensions,
duration, `captured_at`, model, thumbnail path/version/status), and the server
(`CatalogStore` in `src/server/catalog.rs`) reloads its snapshot whenever
`catalog/index.json` changes, keeping the previous snapshot on failure — so
republishing JSON is the entire job; no server change is needed.

## Goals / Non-Goals

Goals: republish identical catalog JSON from state in events-only mode;
never touch the media root, external tools, or thumbnails in that mode; keep
state safety (no `delete_unseen`, no prune, no mount-drop guard) while still
holding the build lock.

Non-goals: picking up media added/removed since the last full scan (the next
full run handles that); changing event validation, JSON formats, or the
server; supporting events-only before any full build.

## Decisions

- **Rebuild source: the state database, not the published shards.** Shards
  carry `thumb_version` only inside the thumbnail URL and omit fingerprints;
  state rows are the authoritative record and `load_items()` already exists.
  Alternative (parse `catalog/*.json`) was rejected: lossy and duplicates the
  load path the server already maintains.
- **Branch point: early in `run_with_config`, after the lock and state open,
  before the tool check and scan.** Events are loaded as today; then the mode
  loads item records, maps them to `CatalogItemInput`, calls
  `build_catalog` -> `attach_events` -> `json::write_json`, and returns. The
  ffmpeg/ffprobe check (`probe::ensure_external_tools`) is skipped in this
  mode, so the update works without the tools installed.
- **Missing state database is a hard error.** `open_read_only`/`open` on an
  absent DB would create or fail confusingly; events-only mode instead checks
  existence first and exits with "run a full build first". An empty catalog
  must never be published.
- **Lock and run bookkeeping retained.** The build lock is still acquired
  (the mode writes JSON that can race a full build). A row is still written to
  the `runs` table via `start_run`/`finish_run` with the republished item
  count, but `last_seen_run` is never updated and `delete_unseen` is never
  called in this mode, so a later full run's cleanup is unaffected.
- **Config plumbing mirrors the existing bools.** `--events-only` on `Cli`,
  `events_only` on `FileConfig`, `events_only: bool` on `Config`, resolved as
  `cli.events_only || file.events_only.unwrap_or(false)`. `Config::validate`
  skips only the media-root *existence* check when the mode is on; the root
  must still be configured and all other validation still runs.
- **Summary marker.** `RunSummary` gains `events_only: bool`; its rendered
  output states that scan/probe/thumbnails were skipped and reports JSON
  files written plus per-event counts. `--force`/`--prune` are accepted but
  have no effect in this mode; `--dry-run` composes and writes nothing.

## Risks / Trade-offs

- [Stale membership] Events-only output reflects the last full scan; media
  added or removed since then appears only after the next full run.
  -> Mitigation: the summary marks the run as events-only; this is inherent to
  skipping the scan and is documented in the flag help.
- [Racing builds] A concurrent full build could republish between the
  events-only read of state and its write. -> Mitigation: the build lock is
  held for the whole run, and JSON writes are atomic (`write_atomic`).
- [Lock file on unmounted state root] If the state root itself is on the
  unmounted NAS, events-only mode cannot run. -> Mitigation: none needed;
  state root is local rebuildable state by design (`storage-roots`).

## Migration Plan

No migration: the flag defaults to off and existing configs are unchanged.
Rollback is simply not passing `--events-only`.

## Open Questions

None blocking. (Whether the `runs` table status for an events-only run should
be a distinct string such as `events-only` instead of `ok` is cosmetic and can
be decided during implementation without affecting the specs.)
