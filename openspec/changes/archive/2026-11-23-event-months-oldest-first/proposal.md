# Proposal

## Why

Unsectioned event pages already list items oldest first within each folder-month
group, but the month groups themselves render newest first. For a multi-month
event this breaks the chronological reading order: June appears above May, even
though May's items come first inside their own group. Month groups should follow
the same oldest-first direction as the items they contain.

## What Changes

- On unsectioned event pages, folder-month groups are ordered oldest first by
  folder year and month (ascending), regardless of how many years the event spans.
- Item ordering within each month group stays oldest first; the renderer follows
  the model's month sequence, so the whole unsectioned event page reads
  chronologically.
- Sectioned event pages are unaffected: they group by authored sections in
  chronological order and do not render month groups.
- Month, year, and index pages keep their existing newest-first month and item
  ordering.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `event-browsing`: the "Render event pages" requirement changes so unsectioned
  event pages order folder-month groups oldest first instead of
  reverse-chronologically, and a new requirement pins month-group order and its
  effect on viewer navigation.

## Impact

- `src/events.rs` (`build_event_views`): emit event month groups in ascending
  year/month order instead of inheriting the catalog's newest-first traversal.
- `src/events.rs` and `src/render.rs` tests plus the `event_may_trip` render
  snapshot: update expectations for month-group order.
- No published JSON shape change: event month groups are internal to the model
  and rendered server-side; `index.json` event metadata is unchanged.
