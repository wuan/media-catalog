# Design

## Context

The catalog model sorts items newest first everywhere (`build_catalog` in
`src/model.rs`), which drives month, year, and index pages. Event pages inherit
that order because `build_event_views` in `src/events.rs` filters items out of
the already-sorted month groups without re-sorting. See proposal.md for why event
pages should read oldest first.

## Goals / Non-Goals

**Goals:**
- Event pages (sectioned and unsectioned) render items oldest first within each
  section, month group, and the Other bucket.
- The in-page viewer follows document order, so it also navigates oldest first on
  event pages — no viewer change needed.

**Non-Goals:**
- Changing item order on month, year, or index pages.
- Changing the order of sections (chronological) or month groups
  (reverse-chronological) on event pages.
- Changing the published JSON shape or the stored catalog model order.

## Decisions

**Sort at event-view build time, not at render time.**
`build_event_views` already assembles the per-event item collections; sorting there
keeps the render layer dumb and makes the order part of the validated model. The
alternative — sorting in the template or render function — would duplicate the
comparator and bypass the count/membership validation that runs on the same data.

**Two sort sites cover all three buckets.**
1. Each unsectioned month group's filtered items are sorted ascending.
2. The flattened all-items list (used for sections and the Other bucket) is sorted
   ascending once; `section_view` and `other_items` filter from it and inherit the
   order.

**Comparator**: ascending `captured_at`, then ascending `path` as the stable
tie-break — the same tie-break as the global newest-first sort, mirrored.

**Alternative considered**: reversing the vectors. Reversal alone would not produce
a stable oldest-first order when timestamps tie (it would reverse the path
tie-break too). An explicit ascending sort is clearer and correct.

**Alternative considered**: changing `build_catalog` to sort ascending globally and
reversing at render time for non-event pages. Rejected: it would touch every page
and risk regressions in month/year/index ordering for no benefit.

## Risks / Trade-offs

- **Snapshot tests** for event pages (`event_portugal_sections`, `event_may_trip`)
  will need updated snapshots since tile order changes. Expected and contained.
- **Existing tests** that assert newest-first order within event sections (e.g.
  `section_views_assign_boundaries_and_collect_other_items`) must be updated to
  expect oldest-first. Expected.

## Migration Plan

No data migration. The change is a pure build-time ordering change; republishing the
catalog with the updated generator produces the new order. Rollback is reverting the
generator change and republishing.

## Open Questions

None.
