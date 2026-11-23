# Design

## Context

See proposal.md - Why. Event views are assembled in `build_event_views`
(`src/events.rs`) by traversing `Catalog::years`, which `build_catalog` stores
newest-first. Each matching month's items are copied and sorted oldest first, but
the month groups inherit the newest-first traversal. The renderer
(`render_event` in `src/render.rs`) emits `EventView::months` in vector order, so
the model's month order is the page order.

## Goals / Non-Goals

**Goals:**
- Unsectioned event pages render folder-month groups oldest first across month
  and year boundaries.
- The event model order matches the rendered item order for unsectioned events.
- Published JSON stays byte-identical for unchanged inputs.

**Non-Goals:**
- Changing `Catalog::years` / per-year `YearView::months` ordering, which drives
  the month, year, and index pages.
- Changing section ordering or tied item ordering rules.
- Changing the JSON schema or store format.

## Decisions

**Order month groups in `build_event_views`, not the template.**
The event view is the validated, deterministic model shared by rendering and
snapshot loading. Sorting there keeps the render layer order-agnostic and avoids
a second ordering rule in templates. Rendering in `build_event_views` also means
the server rebuilds the same order when it loads a published snapshot.

**Sort month groups by `(year, month)` ascending after collection.**
The current loop naturally collects newest-first; an explicit ascending sort on
the completed `months` vector is small, obvious, and independent of catalog
traversal. Alternative: traverse `catalog.years` and `year.months` in reverse.
That also works, but relies on the catalog's ordering invariant and is easier to
break if `build_catalog` changes. Explicit sorting expresses the event-page
requirement directly.

**Leave item sorting and section ordering untouched.**
Items are already sorted ascending within each month and in the flattened list;
sections are already chronological. Only the month-group sequence changes.

## Risks / Trade-offs

- **Snapshot drift** for `event_may_trip` and any test asserting June before May
  → update the render snapshot and assertions; JSON snapshots are unaffected
  because event month groups are not serialized.
- **Multi-year events** could regress if only month numbers are compared →
  sort by `(year, month)`, not month alone; add a multi-year case in tests.
- **Sectioned pages** still compute month groups that are not rendered → keep
  sorting them for model consistency without changing sectioned output.

## Migration Plan

No data migration. Republish the catalog with the updated generator to pick up
the new event page order. Rollback is reverting the ordering change and
republishing; stored JSON is unchanged either way.

## Open Questions

None.
