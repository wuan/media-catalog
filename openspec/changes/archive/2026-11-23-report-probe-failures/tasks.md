# Tasks

## 1. Probe failure summary model

- [x] 1.1 Add `probes_attempted`, `probe_failures: Vec<ProbeFailureGroup { cause, count, samples }>`, and `probe_warning: Option<String>` to `RunSummary` in `src/report.rs`, plus a helper that builds sorted groups (count desc, then cause; max 3 samples each) from `(path, cause)` pairs; verify with a unit test that 500 permission failures and 2 decode failures produce two groups in that order with 3 and 2 samples
- [x] 1.2 Render `Probe failures: F of A` after the `Probed:` line, one `  - <cause>: <count> (e.g. …)` line per group capped at 10 with `... and N more causes`, and `Warning: <probe_warning>` alongside the mount warning; verify with `render()` unit tests for zero failures (`Probe failures: 0 of N`, no cause lines), grouped failures, the cause cap, and the events-only run (no probe lines)

## 2. Record probe failures in the pipeline

- [x] 2.1 In `run()` (`src/lib.rs`), set `summary.probes_attempted` to the probe work size and collect each failed outcome's `error.root_cause().to_string()` with its path into the grouped `probe_failures`, keeping the existing per-file `{error:#}` entry in `summary.errors` and the fallback `captured_at`; verify `cargo test` passes and a pipeline test with one corrupt JPEG reports `Probe failures: 1 of N` and a decode cause
- [x] 2.2 Set `probe_warning` when `probes_attempted >= 4` and failures ≥ 50%, naming the counts and stating that originals are likely unreadable and the catalog will lack dimensions and camera metadata; verify with a pipeline test where 4 of 4 JPEGs are unreadable via `set_readable(false)` (warning present, non-zero exit) and unit-level checks that 3/3 and 2/100 failures produce no warning; skip the permission test when the file stays readable (running as root)
- [x] 2.3 Add a pipeline test in `tests/pipeline.rs`: build with an unreadable JPEG (failure recorded, `width` null in the shard), restore readability without changing size or mtime, rebuild (item re-probed, shard has upright width/height and camera fields, `Probe failures: 0 of 1`), rebuild again (nothing probed); verify it passes with `cargo test --test pipeline`
- [x] 2.4 Document the new probe failure lines and systemic warning in README.md "Run summary and exit status", including that fixing access and re-running re-probes affected items; verify the section matches the rendered output from the 2.2 test

## 3. Integration check

- [x] 3.1 Run `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test`; verify all pass

## Workflow follow-up

- On the deployment, restore the importer's read access to the originals and re-run it; confirm the summary shows `Probe failures: 0 of N` and that `http://pics/catalog/2026.json` items carry `width`/`height`, which restores correct viewer placeholder sizing.
- Archive the change after review.
