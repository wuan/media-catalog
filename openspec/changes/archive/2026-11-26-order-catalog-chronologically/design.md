# Design

## Context

See `proposal.md` - Why. `build_catalog` in `src/model.rs` is the single ordering authority for both rendered HTML and published JSON. It currently iterates years and months in reverse order and sorts items newest first. `render_index`, `render_year`, and `render_month` render vectors in model order, and `render_json` serializes the same order. The server rebuilds its in-memory catalog from the published JSON in file order (`catalog_from_parts` groups consecutive shard items by month), so the published order becomes the served order.

Event pages are separate: `build_event_views` already sorts folder-month groups and items ascending, so they already satisfy the target direction.

## Goals / Non-Goals

**Goals:**
- One archive rule: root years newest first, months within a year ascending by number, items within a month oldest first.
- The rendered pages and published JSON express the same order.
- Event pages keep their chronological narrative order.
- No schema version bump, data migration, or filesystem layout change.

**Non-Goals:**
- Reordering the event overview list, which stays newest start first.
- Changing month previous/next navigation, which is already chronological.
- Making the server re-sort catalogs it loads from old published JSON.
- Adding a separate search ordering rule.

## Decisions

### Sort in `build_catalog`, not in templates or renderers

The model is the deterministic, validated ordering source already used by HTML, JSON, and snapshot loading. Changing it once keeps every consumer consistent and avoids a second comparator in render code or templates.

Alternative considered: sort at render time only. Rejected because published JSON would keep the old order, and the server would render one order while JSON and search expose another.

### Keep the year reversal, drop the month reversal, flip the item comparator

The year loop keeps `.rev()` so the root stays newest first. The month loop iterates ascending. The item sort changes from descending `captured_at` to ascending `captured_at`, keeping the path tie-break ascending so repeated runs remain deterministic.

Alternative considered: reverse the month item vectors. Rejected because reversal would also reverse the path tie-break for equal timestamps; an explicit ascending comparator mirrors the existing contract.

### Leave event views untouched

`build_event_views` traverses the catalog's years, collects matching month groups, then sorts groups by `(year, month)` ascending and items ascending. That behavior is already the event contract and does not depend on the catalog's month traversal order. Keeping it untouched means event pages remain chronological even though the root year list is newest first.

### Do not normalize old catalogs on load

The server will keep rendering whatever order the published JSON stores. Old catalogs therefore keep serving newest-first month lists and items until they are republished. Re-sorting on load would hide stale output, duplicate the generator's ordering rule in the server, and complicate validation and byte-identical snapshot expectations.

Alternative considered: sort in `catalog_from_parts`. Rejected for the reasons above; the generator is the ordering authority, and republishing is the migration step.

### Let search inherit the archive order

Search already concatenates shard items in published order. It will show newest years first, then ascending months and oldest items within each month, matching the browsing hierarchy. Adding a client-side sort would introduce a second ordering rule and diverge from the archive pages.

Alternative considered: sort search results newest-first by capture time. Rejected to keep one predictable rule across the catalog.

## Risks / Trade-offs

- **Published JSON order changes** → Document the new order and mark it breaking for downstream consumers; the schema shape is unchanged, so no version bump is required.
- **Visitors with an old published catalog** → They keep seeing the old order until the next build; the sample deployment must republish as part of rollout.
- **Non-monotonic year traversal** → From December 2024 the "next month" link can reach January 2025, which is listed above 2024 in the root index. This follows the chosen rule (newest years first, months ascending) and is accepted; month navigation itself stays chronological.
- **Multi-year event pages** → Event month groups remain oldest year first, which differs from the root year order but preserves the event narrative.
- **Snapshot and test churn** → Update model, render, JSON, search, pipeline, and snapshot expectations in the same change; the diffs should show only reordered months and items.
- **Search expectations** → Search tests and documentation should describe the inherited archive order, not a separate newest-first result order.

## Migration Plan

Deploy the updated generator and rebuild the catalog. The new `catalog/index.json` and year shards carry the new order; the server picks it up on reload. Existing catalogs keep serving their stored order until rebuilt, so roll forward by republishing rather than by changing the server. Rollback is reverting the generator ordering change and republishing; stored JSON has no schema or data migration either way. An events-only republish also produces the new order because it rebuilds the catalog model from persisted state.
