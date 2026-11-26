# Proposal

## Why

Catalog overview pages currently read in different directions: the root and year
pages list years and months newest first, month grids show newest items first,
while event pages already read oldest first. Visitors browsing an event's month
link or moving from a year page into a month see the order flip. The catalog
should have one predictable reading direction for its hierarchy: newest year
first, but each year laid out January through December and each month's items
oldest first.

## What Changes

- Keep root/index years ordered newest first.
- Change year pages so months are ordered by month number ascending (`01` through
  `12`), skipping months that contain no items.
- Change month pages so items are ordered oldest first by capture timestamp with
  the relative path as a stable tie-break.
- **BREAKING**: `catalog/index.json` keeps years newest first but publishes each
  year's `months` array ascending; `catalog/YYYY.json` shards group months
  ascending and list items oldest first. The JSON schema is unchanged, but
  consumers that rely on the previous reverse-chronological order will see a
  different order.
- Leave event pages unchanged: their authored sections and folder-month groups
  already render oldest first, and their narrative order remains chronological.
- Search results inherit the catalog order from the shards; no separate search
  ordering rule is introduced.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `media-inventory`: the deterministic ordering requirement changes from
  reverse-chronological years, months, and items to newest-first years,
  ascending months, and oldest-first items within a month.
- `event-browsing`: the scenarios that pin non-event month, year, and index pages
  to newest-first item order change to the new hybrid archive order.

## Impact

- `src/model.rs` (`build_catalog`): keep the year reversal, iterate months
  ascending, and sort items ascending.
- `src/json.rs` output order follows the model; `src/server/catalog.rs` and
  `src/render.rs` continue rendering the published order.
- `src/events.rs` and the event renderer stay unchanged; event-page item and
  month-group ordering is already oldest first.
- Model, render, JSON, search, pipeline, and snapshot tests need updated
  expectations for month lists and item order.
- `README.md` and `docs/server.md` update their ordering description.
- No schema version bump, data migration, or filesystem layout change. Existing
  published catalogs keep serving their stored order until republished.
