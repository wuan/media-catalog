# Proposal

## Why

Event pages currently inherit the global newest-first item ordering, so within each
event section (or month group) the latest photos appear at the top. For a curated
event such as a trip, the natural reading order is chronological: the oldest items
first, telling the story from start to finish. Sorting event pages oldest-first
makes sections read as a timeline instead of a reverse timeline.

## What Changes

- On event pages only, items within each authored section, each folder-month group,
  and the "Other" bucket are sorted oldest first (ascending capture timestamp, with
  the relative path as a stable tie-break).
- Month, year, and index pages keep the existing newest-first ordering.
- The in-page viewer's prev/next navigation follows document order on event pages,
  so it also moves oldest-to-newest.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `event-browsing`: the "Render event pages" requirement gains item ordering within
  sections, month groups, and the Other bucket; a new requirement pins event-page
  item order to oldest-first while non-event pages stay newest-first.

## Impact

- `src/events.rs` (`build_event_views`, `section_view`): sort event-page item
  collections ascending by `captured_at` with path tie-break.
- `openspec/specs/event-browsing/spec.md`: delta spec for the new ordering.
- No changes to the viewer script, templates, month/year/index rendering, or the
  published JSON shape.
