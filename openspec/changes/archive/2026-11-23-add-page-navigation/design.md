# Design

## Context

See `proposal.md` - Why. Month pages are rendered on demand from the in-memory `Catalog` (`src/render.rs`), whose `years` and `months` vectors are stored newest first. Month pages exist only for months that contain items, so month numbers are sparse and month pages do not align with calendar adjacency. Event pages render their groups from the same catalog view: authored sections plus an optional `Other` bucket, or folder-month groups for unsectioned events. All pages are plain server-rendered HTML with no required JavaScript.

## Goals / Non-Goals

**Goals:**

- Resolve previous/next month pages from existing catalog data without changing the published JSON contract.
- Keep navigation robust at gaps, year boundaries, and catalog edges.
- Make every link a plain anchor with a stable, testable HTML shape.
- Keep month, year, and index item ordering untouched.

**Non-Goals:**

- Adding year-level previous/next navigation or a new route.
- Pagination, infinite scroll, or client-side navigation.
- Changing event grouping, section ordering, or viewer navigation.
- Persisting navigation links in `catalog/index.json`.

## Decisions

### Derive neighbours in the renderer, not the published data

`render_month` will compute the nearest earlier and later month pages directly from `Catalog` and pass them to `MonthTemplate` as optional link views. The catalog is already loaded for rendering, and navigation is a presentation concern, so no JSON field, validation rule, or reload path changes.

**Why:** publishing `prev`/`next` would create a second source of truth that must stay consistent with the shard set, and it would alter deterministic JSON output for a purely visual concern.

**Alternatives considered:** adding navigation fields to `index.json` (couples presentation to the data contract and forces validator changes), and computing neighbours in `viewer.js` (breaks the no-JavaScript requirement).

### Chronological neighbours across the whole catalog

Flatten the catalog's year/month pairs into ascending chronological order, find the current page, and take the immediate neighbours. This crosses year boundaries and skips months with no page. A missing neighbour is omitted rather than rendered as a disabled or `404` link.

**Why:** a month page is a browsing position in the catalog's timeline, not a calendar slot. Linking to a page that does not exist would strand the visitor, and linking only within a year would dead-end at January and December even when adjacent pages exist.

**Alternatives considered:** calendar `month - 1` / `month + 1` (simple, but produces dead links at gaps), same-year-only neighbours (never crosses years, dead-ends at year edges), and querying the filesystem for neighbouring folders (duplicates catalog state and ignores items outside the scan).

### Shared navigation markup, repeated at top and bottom

The month navigation is one small markup fragment included twice in `templates/month.html`: once after the breadcrumbs and once after the tile grid. It uses the same `prev_month` / `next_month` fields at both positions, with `rel="prev"` and `rel="next"`, and a flex layout that keeps the next link on the right when the previous link is absent.

**Why:** long month grids can exceed a viewport many times over; without a bottom copy the visitor must scroll back up to leave the month. A shared fragment keeps both copies identical so tests can assert one shape.

**Alternatives considered:** top-only navigation (simpler, but poor on long months) and a floating/fixed control (more intrusive and unnecessary for a static catalog).

### Full month-and-year labels

Each link uses the catalog's `label` (`April 2024`), not just the month name or an arrow. The year stays visible when navigation crosses a boundary.

**Why:** `May 2024` -> `December 2023` is unambiguous, while `December` alone is not.

**Alternatives considered:** compact labels with the year only when it changes (less repetitive, but two rendering shapes and more conditional logic).

### Back-to-top anchors at the end of each event group

`templates/event.html` adds `id="top"` to the page and renders a `Back to top` link after the tile grid of each group: `Other`, each authored section, and each `month-section` on unsectioned events. The link targets `#top` with a plain anchor.

**Why:** the section navigation lives at the top; after scrolling through a long group the visitor needs a direct route back to it without relying on browser chrome or JavaScript. `#top` is native browser behavior and survives the no-JavaScript path.

**Alternatives considered:** placing the link in each heading (visible before the group, but still mid-page), a fixed floating button (obscures tiles on small screens), and JavaScript scroll-to-top (fails the no-JavaScript requirement).

### Renderer-first tests and snapshots

Add explicit render tests for cross-year neighbours, skipped gaps, catalog edges, and the single-month case, plus assertions that each event group carries exactly one back-to-top link after its grid. Update the existing month and event snapshots.

**Why:** navigation is generated HTML, so string-level assertions cover the observable contract without a browser. Snapshot updates make the markup change reviewable.

**Alternatives considered:** browser/e2e tests (heavier and not currently part of this project's test surface).

## Risks / Trade-offs

- **Cross-year jumps may surprise visitors** -> full month-and-year labels make the destination explicit, and `rel="prev"`/`rel="next"` conveys direction to assistive technology.
- **Duplicate navigation markup** -> a shared include keeps the two copies byte-identical; tests assert both positions.
- **Snapshot churn** -> expected and limited to month and event snapshots; the diff should show only the added navigation fragments.
- **Back-to-top links add height to every event group** -> the link is a single line after the grid, styled muted, so it stays visually subordinate to the tiles.
- **Neighbour lookup cost** -> a single linear pass over catalog months per request; the catalog is already in memory and month pages are not a hot path.

## Migration Plan

No data migration. Deploying the updated server changes only rendered HTML; published JSON, routes, and configuration stay compatible. Reverting the change restores the previous HTML output without touching catalog data.
