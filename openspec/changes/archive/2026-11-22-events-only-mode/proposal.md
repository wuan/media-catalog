# Proposal

## Why

Editing `events.toml` currently requires a full generator run, and on a large
archive the scan, probe, and thumbnail phases dominate the wall-clock time —
even when nothing on disk changed. Curating events (adding, retitling,
re-ranging, or adding sections) therefore feels disproportionately slow, and
is impossible while the media root is unmounted, because the run fails before
reaching the publish step.

## What Changes

- Add an `--events-only` flag to the generator and a matching `events_only`
  key in the shared `media-catalog.toml` schema (CLI wins over TOML, like the
  other boolean settings; the server parses and ignores the key).
- In events-only mode the generator skips scanning, fingerprint diffing,
  probing, thumbnail generation, state cleanup (`delete_unseen`), pruning,
  the mount-drop guard, and the external-tool check. It still takes the build
  lock, then rebuilds the catalog from the persisted state database, reattaches
  the authored events, and republishes the catalog JSON atomically.
- The media root must still be configured, but its existence check is skipped
  in this mode only, so event updates work while the archive is unmounted.
- With no state database present, events-only mode fails with a clear error
  telling the user to run a full build first; it never publishes an empty
  catalog.
- `--dry-run` combined with `--events-only` validates the events file and
  reports what would be republished without writing. `--force` and `--prune`
  have no effect in this mode.
- The run summary marks the run as events-only and reports the republished
  JSON file count and per-event item counts.
- No server changes: the server already reloads its catalog snapshot when
  `catalog/index.json` changes and keeps the previous snapshot on a failed
  reload.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `configuration`: new `--events-only` flag and `events_only` TOML key in the
  shared schema; the server accepts and ignores the key.
- `event-browsing`: new requirement that events can be republished quickly
  from persisted state without scanning, probing, or thumbnail work.
- `incremental-builds`: events-only runs SHALL NOT delete unseen state rows,
  prune thumbnails, or trigger the mount-drop guard; the state database is the
  rebuild source in this mode.

## Impact

- `src/config.rs`: `Cli`, `FileConfig`, `Config` gain the `events_only`
  setting; validation of media-root existence is relaxed in events-only mode.
- `src/lib.rs`: `run_with_config` branches into the events-only fast path
  before scanning.
- `src/report.rs`: `RunSummary` marks events-only runs in its rendered output.
- `README.md`: document the flag and its semantics.
- Tests: config resolution, the fast-path pipeline (state -> JSON republish,
  no state mutation, lock still held, missing-state error), and dry-run
  interaction.
