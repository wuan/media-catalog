# Tasks

## 1. Event configuration and validation

- [x] 1.1 Add an `events.toml` loader with `slug`, `title`, `start`, `end`, and optional `description`, and verify parsing tests cover a valid file, unknown fields, and a missing explicit file.
- [x] 1.2 Add the `--events` flag and default `./events.toml` discovery, and verify configuration tests cover default, explicit, absent-default, and missing-explicit cases.
- [x] 1.3 Validate slug format and uniqueness, required title, parseable `YYYY-MM-DD HH:MM` timestamps, and `start <= end`, and verify each failure has a test and a clear error message before scanning.
- [x] 1.4 Document `events.toml`, the `--events` flag, and event field semantics in `README.md`, and verify every documented command and example matches the implemented behavior.

## 2. Catalog model and membership

- [x] 2.1 Extend the catalog model with event definitions, per-item event slugs, an internal item year, and event views grouped by month, and verify existing model tests still pass.
- [x] 2.2 Implement membership assignment by effective timestamp truncated to the minute with inclusive start and end, and verify tests cover both boundaries, seconds past the end minute, outside-range items, cross-month events, overlapping events, and the mtime fallback.
- [x] 2.3 Verify that folder grouping stays authoritative for year and month browsing while event membership follows the timestamp, including an item stored under `2024/06` with a May capture timestamp.
- [x] 2.4 Verify deterministic ordering: events sorted by newest start then slug, month groups newest first, items newest first, and item event slugs sorted independently of config order.

## 3. JSON publication and snapshot validation

- [x] 3.1 Add optional `events` metadata to `catalog/index.json` and optional `events` slug lists to year shard items, omitting both when empty, and verify a catalog without events remains byte-identical to the existing snapshots.
- [x] 3.2 Add JSON snapshot tests for event metadata and item membership, and verify repeated rendering produces byte-identical event output.
- [x] 3.3 Rebuild event views in `catalog_from_parts` and validate unique declared slugs, undeclared item slugs, and declared counts against shard membership, with tests for each rejection and a successful round trip.
- [x] 3.4 Document the published event JSON fields and an example in `README.md`, and verify the example matches generated output.

## 4. Server routes and rendering

- [x] 4.1 Register `/events/` and `/events/{slug}/`, confirm they take precedence over numeric year/month routes, and verify `503` before publication, `404` for unknown slugs, and ETag revalidation tests.
- [x] 4.2 Add the Events section above Years on `/` and an event list page ordered by newest start then slug, with tests for a populated root, a populated list, and an empty list.
- [x] 4.3 Render the event page with title, date range, optional description, item count, and month-grouped sections that link headings to `/YYYY/MM/`, and verify month order, item order, lazy images, video markers, placeholders, escaping, empty events, and no-JavaScript markup.
- [x] 4.4 Document the event routes, root navigation, and event page behavior in `README.md` and `docs/server.md`, and verify the documented URLs resolve as described.

## 5. Run reporting and guardrails

- [x] 5.1 Report each configured event's matched item count in the run summary and add a warning when an event exceeds 500 items, with tests for normal and large-event output.
- [x] 5.2 Document the event reporting and large-event guidance, and verify the warning text and threshold match the documented behavior.

## 6. Integration verification

- [x] 6.1 Add an end-to-end test that builds a catalog from a media tree and `events.toml`, starts the server, and verifies the event list, a cross-month event page, and the unchanged year/month pages.
- [x] 6.2 Run `cargo fmt --check`, `cargo test`, and `cargo clippy -- -D warnings`, and verify all three pass.
- [x] 6.3 Verify a catalog without `events.toml` still publishes byte-identical JSON and that a catalog without event fields loads as having no events.

## Workflow follow-up

- Archive the change after the project's review requirements are satisfied.
- Verify the archived result.
