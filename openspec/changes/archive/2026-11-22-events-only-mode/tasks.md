# Tasks

## 1. Configuration plumbing

- [x] 1.1 Add `--events-only` to `Cli`, `events_only` to `FileConfig`, and `events_only: bool` to `Config` with CLI-over-TOML resolution in `src/config.rs`; verify with new unit tests in `src/config.rs` covering flag, TOML key, flag-wins, and server-side parsing of the shared file (`cargo test config`)
- [x] 1.2 Skip only the media-root existence check in `Config::validate` when `events_only` is set (root still required as a setting, all other validation unchanged); verify with unit tests for both modes in `src/config.rs` (`cargo test config`)

## 2. Events-only fast path in the pipeline

- [x] 2.1 Add the `ItemRecord` -> `CatalogItemInput` mapping helper and branch in `run_with_config` (`src/lib.rs`): after the lock and state open, when `events_only` is set, fail with a clear error if the state database is missing, otherwise load items, rebuild the catalog, attach events, publish JSON via `json::write_json`, finish the run, and return — skipping the tool check, scan, diff, probe, thumbnails, `delete_unseen`, prune, and mount-drop guard; verify with a new test in `tests/pipeline.rs` that runs a full build, edits the events file, runs `run_with_config` with `events_only: true`, and asserts the republished `catalog/index.json` carries the new event while `state.sqlite` rows (thumb status, `last_seen_run`) are unchanged
- [x] 2.2 Verify the fast path never reads the media root or requires ffmpeg: integration test in `tests/pipeline.rs` that removes/renames the media root (and runs without ffmpeg on PATH where the harness allows) and asserts an events-only run still republishes byte-identical JSON to the previous full build
- [x] 2.3 Cover the edge cases in `tests/pipeline.rs`: events-only with no state database exits non-zero and writes nothing; `--events-only --dry-run` validates events and writes no output or state; `--events-only --prune` prunes nothing; invalid events file fails before publishing; the build lock is still held during an events-only run (second concurrent run is refused)

## 3. Run summary reporting

- [x] 3.1 Add `events_only: bool` to `RunSummary` in `src/report.rs` and render it in the summary output (states scan/probe/thumbnails were skipped, JSON files republished, per-event counts); verify with a unit test on the rendered summary and the pipeline test from 2.1 (`cargo test report` plus `cargo test --test pipeline`)

## 4. Documentation

- [x] 4.1 Document `--events-only` / `events_only` in `README.md` (flag list and events section, including the unmounted-media-root and missing-state behaviors); verify the documented commands match `--help` output and that `cargo test` still passes (README examples are exercised by tests)

## 5. Integration verification

- [x] 5.1 Run the full suite and linters: `cargo test`, `cargo fmt --check`, `cargo clippy -- -D warnings`; verify all pass with the new tests included
