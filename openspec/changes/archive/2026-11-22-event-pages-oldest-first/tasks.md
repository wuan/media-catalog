# Tasks

## 1. Sort event-page items oldest first

- [x] 1.1 In `build_event_views` (src/events.rs), sort each unsectioned month group's filtered items ascending by `captured_at` with path tie-break; verify with `cargo test events::` that unsectioned event tests compile and the ordering assertions are updated to expect oldest first
- [x] 1.2 Sort the flattened all-items list ascending in `build_event_views` so `section_view` and `other_items` inherit oldest-first order; verify with `cargo test events::` that section and Other bucket assertions expect oldest first
- [x] 1.3 Update existing tests in src/events.rs (e.g. `section_views_assign_boundaries_and_collect_other_items`, `events_and_items_are_ordered_deterministically`) to expect oldest-first item order on event pages; verify with `cargo test events::`

## 2. Update render tests and snapshots

- [x] 2.1 Update render tests in src/render.rs that assert item order on event pages to expect oldest first; verify with `cargo test render::`
- [x] 2.2 Regenerate insta snapshots for event pages (`event_portugal_sections`, `event_may_trip`) and verify with `cargo test render::` that snapshots match

## 3. Integration check

- [x] 3.1 Run the full test suite with `cargo test` and verify all tests pass, confirming month/year/index pages remain newest-first and event pages are oldest-first
