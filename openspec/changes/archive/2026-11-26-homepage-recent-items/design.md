# Design

## Context

See `proposal.md` for motivation. Current constraints that shape the approach:

- `CatalogSnapshot` already loads `catalog/index.json` and every referenced year shard into memory, so the server has all item records for any root-page request.
- Catalog ordering is folder-first: years newest first, months ascending, items oldest first within a month. Capture timestamps can disagree with folder position, so folder order cannot answer "most recent".
- `tile_from_item` and `templates/tiles.html` already produce the canonical tile markup and viewer data attributes used on month and event pages.
- The tile grid is `repeat(auto-fill, minmax(9rem, 1fr))`, so column count is viewport-dependent and unknown to the server.
- `/` currently loads neither `viewer.js` nor the location-preview body attribute.

## Goals / Non-Goals

**Goals:**

- Surface the newest catalog items on `/` without touching the generator, published JSON, state schema, or configuration.
- Reuse the existing tile rendering path so the strip is visually and behaviorally consistent with month pages.
- Keep root-page output deterministic for a given catalog snapshot.
- Make the strip a working in-page viewer album with accessible, no-JavaScript fallback links.

**Non-Goals:**

- A dedicated "all recent" or paginated recent page.
- Changes to `catalog/index.json` or year shards.
- Per-user personalization, favourites, or ranking other than recency.
- Guaranteeing exactly four rows at every viewport; four rows is the desktop rendering of a fixed 24-item set.

## Decisions

### Derive recents at render time instead of publishing them in JSON

`render_index` selects items from `&Catalog`. Alternative: add a `recent` array to `catalog/index.json` at build time. Rejected because the strip is presentation-only, would duplicate item records in the commit-point file, force a JSON schema bump and snapshot-validation changes, and require a generator rebuild for a server-only concern. Trade-off: the root render sorts all loaded items per request; this is acceptable at catalog scale and can be precomputed in the snapshot later without an external interface change.

### Order by global capture timestamp, then relative path descending

"Most recent" means the newest capture timestamps across the whole catalog, not the tail of the newest shard. Folder position and EXIF/video capture time can disagree, and event membership already follows capture time. Ties resolve by descending relative path so the order is deterministic and respects the newest-first intent. Alternative: capture timestamp descending with path ascending; rejected because it makes tie ordering feel arbitrary next to newest-first.

### Fixed 24 items with a six-column grid

24 = 4 rows x 6 columns at desktop width. Below the desktop breakpoint the same fixed set reflows to three and two columns, giving more rows without clipping, hiding, or removing tiles from the tab order. Alternatives rejected: a strict four-row shelf with horizontal scrolling (unusual homepage interaction and poorer keyboard browsing), and JavaScript-driven item counts (server-rendered catalog pages must work without JavaScript).

### Reuse `tile_from_item` and `tiles.html` unchanged

The strip includes the existing caption, alt text, thumbnail or placeholder, video badge, and viewer `data-*` attributes. This keeps one tile contract and lets the viewer's document-wide tile discovery work on `/` without viewer code changes.

### Load the viewer and location preview on the root page

`templates/index.html` gains `viewer.js` and the `data-location-preview` attribute driven by `catalog.location_preview`, matching month and event pages. If the script fails or the browser lacks dialog support, tiles remain ordinary links to `/media/`.

### Keep the section empty-safe and snapshot-keyed

Fewer than 24 items renders what exists; an empty catalog renders no `Recent` heading. The root page ETag stays `snapshot id + route`, so the strip invalidates exactly when the published catalog changes.

## Risks / Trade-offs

- [Sorting all items on every root render] -> Catalog-scale sort is cheap and root renders are ETag-revalidated; if profiling shows pressure, precompute the 24-item selection once per snapshot.
- [Homepage loads 24 tiles] -> Existing lazy loading and async decoding apply; the strip is the visual point of the page, so this is intended.
- [Viewer album is only the 24 recent tiles, not the archive] -> Matches the established per-page album scope; navigation stops at the strip's edges.
- [Four rows is desktop-only] -> Deliberate responsive reflow; the spec requires reflow without hiding tiles, not a fixed row count on mobile.
- [Capture-time fallbacks can misrank files with missing EXIF] -> The catalog already treats modification time as the effective capture time; recents inherit that documented behavior.

## Migration Plan

No data migration. Deploy the server build with the updated renderer, template, and stylesheet; the published catalog format is unchanged. Rollback is reverting the server binary and assets; existing JSON and thumbnails remain valid.
