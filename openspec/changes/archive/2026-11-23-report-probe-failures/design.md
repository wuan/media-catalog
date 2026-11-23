# Design

## Context

Probing runs in parallel in `run()` (`src/lib.rs`). Each `ProbeOutcome` carries an `anyhow::Result<ProbeResult>`. A failure stores a fallback `ProbeResult` that has only `captured_at` (the mtime). It then pushes a per-path entry formatted with `{error:#}` into `summary.errors`. Those messages include the file path and context, e.g. `opening image /media/…/x.jpg: Permission denied (os error 13)`, so no two lines look alike even when they share one cause. `RunSummary::render()` (`src/report.rs`) prints the first 25 errors with no categories. The probe phase line `Probed N items (M failed)` goes only to the progress output, not to the final summary.

Retrying already exists. The probe filter selects unchanged rows with `captured_at IS NULL` or, for decodable extensions, `width IS NULL`. A failed decodable probe therefore gets retried on every run. For background, see proposal.md, "Why", and the delta specs for the required behavior.

## Goals / Non-Goals

**Goals:**
- Make a systemic probe failure obvious in the final summary, which is the part cron mails and logs keep.
- Group failures without paths, so causes are comparable.
- Lock in the existing retry behavior with a test.

**Non-Goals:**
- Changing exit codes, the state schema, or the JSON catalog schema.
- Blocking or skipping publication when probing fails. Considered below.
- Retrying failed probes of non-decodable images (HEIC): a successful metadata-only probe can legitimately leave `width` empty, so there's nothing to tell a failure from a success without a schema change.

## Decisions

1. **Group by the anyhow root cause.** The cause key is `error.root_cause().to_string()`, e.g. `Permission denied (os error 13)` or the decoder's message. The full `{error:#}` text stays in `summary.errors`, so the per-file detail is still there. *Alternative:* strip paths from the formatted message with string matching. That breaks easily, because context strings embed paths in different places.

2. **Separate probe stats in `RunSummary`.** Add `probes_attempted: usize` and `probe_failures: Vec<ProbeFailureGroup { cause, count, samples: Vec<String> }>`, sorted by count (highest first) then cause, with up to 3 samples per group. `render()` prints `Probe failures: F of A` after `Probed:`, then one line per cause: `  - <cause>: <count> (e.g. <path>, <path>, <path>)`. To keep long outputs readable, show at most 10 causes, then `... and N more causes`. The existing generic `Errors:` block stays as it is, because thumbnail and publish errors still need it.

3. **Systemic threshold: at least 4 attempts and failures ≥ 50%.** This matches the mount-drop guard's shape (`previous >= 4`, a halving ratio) for consistency and keeps one-off failures in tiny libraries from triggering it. The warning goes into a new `probe_warning: Option<String>`, printed as `Warning: …` next to the mount warning. *Alternative:* a configurable threshold. Not worth adding a config field until someone needs it.

4. **Warn, don't block.** *Alternative:* treat systemic probe failure like a mount drop and skip publishing unless `--allow-probe-failures` is passed. This would have kept the previous good catalog on the deployment. It was rejected for this change: the user chose to surface failures, and blocking changes publication behavior. It can come in a later change if the warning isn't enough.

## Risks / Trade-offs

- [Root-cause strings from different platforms or decoders split one problem into several groups] → Showing samples per group still makes the shared cause clear. The grouping is a reporting aid, not a contract.
- [The permission-based pipeline test is meaningless when run as root (e.g. some CI containers)] → Skip that test when the unreadable file can still be opened after `set_readable(false)`, the same as any other permission-dependent test.
- [The summary gets longer] → Causes are capped at 10 and samples at 3. With no failures the extra output is the single `Probe failures: 0 of N` line.

## Migration Plan

No data migration. Deploy the new importer and re-run it once the originals are readable. Affected items get probed again automatically (decodable extensions), and the summary's new lines confirm `Probe failures: 0 of N`. To roll back, revert the binary. Nothing persistent changes.
