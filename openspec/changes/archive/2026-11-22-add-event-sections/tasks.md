# Tasks

## 1. Section authoring model

- [x] 1.1 Add an authored section entry to `src/events.rs` with `slug`, `title`, `at`, and optional `description`, parsed from `[[events.sections]]`; verify `cargo test events::tests` covers valid parsing and rejection of unknown fields.
- [x] 1.2 Validate authoring rules: section slug format, non-empty title, `at` in `YYYY-MM-DD HH:MM` form, containment within the parent event, unique slugs, and unique timestamps; verify tests reject each invalid case before scanning.
- [x] 1.3 Add resolved section types to `src/model.rs` and carry them on `EventMeta`/`EventView`; verify existing event and model tests still pass.
- [x] 1.4 Document bookmark authoring, section slugs, and the inclusive boundary rule in the README Events section; verify the documented example parses with the new parser.

## 2. Resolve and publish section ranges

- [x] 2.1 Implement bookmark-to-range resolution in `src/events.rs`: `start = at`, non-last `end = next at - 1 minute`, last `end = event.end`; verify unit tests cover boundary assignment, last-section behavior, and a full partition.
- [x] 2.2 Add an optional `sections` array to `IndexEvent` in `src/json.rs` with `slug`, `title`, `start`, `end`, and `description`, omitted when empty; verify round-trip tests preserve section data.
- [x] 2.3 Confirm catalogs with no sections and events with no sections remain byte-identical; verify index and shard snapshots plus the repeat-build test.
- [x] 2.4 Document the published section JSON shape in the README JSON section; verify the example matches the generated snapshot.

## 3. Server validation and section views

- [x] 3.1 Build section views in `build_event_views` from published ranges and item timestamps; verify tests assign an item at a bookmark boundary to the later section.
- [x] 3.2 Validate published sections during snapshot load: unique slugs within an event, parseable timestamps, contained ranges, ascending order, and no overlap; verify a bad index test keeps the previous snapshot serving.
- [x] 3.3 Collect items outside every section into an Other bucket; verify tests cover items before the first section, items in a gap, and omission when the bucket is empty.

## 4. Rendering

- [x] 4.1 Extend `EventTemplate`, `render_event`, and `templates/event.html` with in-page section navigation, section headings, section tiles, and the Other bucket; verify the event page snapshot.
- [x] 4.2 Preserve month-grouped rendering for events without sections; verify existing event snapshots remain unchanged.
- [x] 4.3 Omit empty sections from content and navigation and keep sectioned pages usable without JavaScript; verify tests cover both behaviors.
- [x] 4.4 Update README route/page documentation to describe sectioned event pages and the Other bucket; verify documented behavior matches the rendering tests.

## 5. Integration checks

- [x] 5.1 Run `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test`; verify all pass.
- [x] 5.2 Run `openspec validate add-event-sections --strict`; verify the change is valid.
- [x] 5.3 Generate a fixture catalog with a multi-section Portugal event and serve it; verify `/events/{slug}/` shows section navigation, range headings, and section-grouped tiles.

## Workflow follow-up

- Archive the change after implementation and review are complete.
- Verify the archived event-browsing spec contains the new section requirements.
