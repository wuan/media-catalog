# Tasks

## 1. Order event month groups oldest first

- [x] 1.1 In `build_event_views` (`src/events.rs`), sort the assembled event `months` vector ascending by `(year, month)` so month groups no longer inherit the catalog's newest-first traversal; verify with a targeted unit test that an unsectioned event over May and June exposes `[2024-05, 2024-06]`
- [x] 1.2 Update `events_and_items_are_ordered_deterministically` in `src/events.rs` to expect May before June and adjust the month/item indexes it asserts; verify with `cargo test events::`
- [x] 1.3 Add a multi-year unsectioned event case to the `src/events.rs` tests confirming month groups sort by year then month ascending; verify with `cargo test events::`

## 2. Update rendering expectations and snapshots

- [x] 2.1 Rename/update `event_page_groups_months_newest_first_and_links_month_pages` in `src/render.rs` to assert the May heading and May item appear before June, while month-page links remain intact; verify with `cargo test render::`
- [x] 2.2 Regenerate the `event_may_trip` insta snapshot (`INSTA_UPDATE=always cargo test render::tests::snapshot_event_page`) and confirm the accepted snapshot shows May before June; verify with `cargo test render::`
- [x] 2.3 Update the event-page ordering description in `README.md` and `docs/server.md` to say folder months appear oldest month and oldest item first; verify the rendered statements match the new behavior

## 3. Integration check

- [x] 3.1 Run `cargo test` and confirm all tests pass, including JSON snapshots unchanged and month/year/index pages still newest-first
- [x] 3.2 Run `openspec validate event-months-oldest-first --strict` and confirm the change validates

## Workflow follow-up

- Archive the change after implementation and review are complete.
