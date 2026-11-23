# Proposal

## Why

The viewer's thumbnail placeholder looked mis-sized on the deployed catalog: landscape photos came out far too small and portrait photos slightly too big. Two earlier viewer fixes did not help, because the viewer was never the cause. Every item in the published catalog has `width`/`height` set to `null`. Camera fields and video durations are also missing, and `captured_at` is the file mtime. These are the values the importer stores when a probe fails (`src/lib.rs`, probe outcome handling). With no dimensions, the viewer falls back to showing the 512 px thumbnail at its natural size.

The same original (`2026/11/20261104_160414_FUJIFILM_X-H2S.jpg`) probes correctly with the current code (6240×4160, X-H2S, f/4, ISO 160). So every probe failed in the importer's environment, most likely when the v2→v3 state migration forced every item to be probed again. The run did report errors, but it showed only the first 25 per-file lines among all errors. Nothing said that probing as a whole had failed, so a catalog with no metadata at all was published without anyone noticing.

## What Changes

- The run summary gets a dedicated probe line, `Probe failures: <failed> of <attempted>`, separate from thumbnail and other errors.
- Probe failures are grouped by cause, using the normalized root error (e.g. `Permission denied (os error 13)`). Each group shows its count and a few sample paths, so one environment problem across 70,000 files reads as one line instead of 25 arbitrary entries.
- When most attempted probes fail (systemic failure), the summary prints a prominent warning. The warning says the environment probably can't read the originals and that the published catalog will lack dimensions and camera metadata.
- Items whose probe failed are probed again on the next run, so fixing the environment and re-running is enough to recover. This already happens for decodable images and videos and gets locked in with a regression test.
- Out of scope: blocking publication on systemic probe failure, and any viewer changes. With dimensions restored, the existing placeholder sizing already behaves correctly, verified in Chromium against real JPEG originals and WebP thumbnails.

## Capabilities

### New Capabilities

_None._

### Modified Capabilities

- `media-inventory`: "Tolerate unreadable items" now requires probe failures to be counted and grouped by cause in the run summary, with a warning when probing fails systemically.
- `incremental-builds`: adds a requirement that items whose last probe failed are probed again on later runs until a probe succeeds.

## Impact

- `src/report.rs`: `RunSummary` gets probe-failure fields, and `render()` gets the new probe lines and the systemic warning.
- `src/lib.rs`: records the probe attempt/failure counts and grouped causes, and decides when to warn about systemic failure.
- Tests: unit tests for grouping and rendering in `src/report.rs`, and a pipeline test in `tests/pipeline.rs` (unreadable file, then readable, then re-probed with dimensions).
- No changes to the state schema, JSON catalog schema, server, or viewer. Exit status is unchanged: any error still exits non-zero.
- Operational follow-up (not code): fix the importer's access to the originals on the deployment and re-run the importer. It will probe the affected items again.
