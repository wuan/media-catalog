# Design

## Context

See proposal.md for motivation and specs/ for the required behavior.

How things work today:
- `scan::ScannedFile.size` and `state::ItemRecord.size` (column `items.size`) already hold each original's byte size. Thumbnail versioning uses it, but it stops at `CatalogItemInput` and never reaches `ItemView`, the JSON shards, or the HTML.
- The server renders pages from the published JSON shards (`media-server` "Render catalog pages from published JSON"). The search page builds its tiles client-side from the same shards (`assets/search.js`). So the size has to be in the shards to reach either page.
- The viewer builds its item list from the tile anchors (`tileFromElement` in `assets/viewer.js`). It reads `data-captured` and the `.model` caption span, and formats the caption line in `captionText(item)`.

## Goals / Non-Goals

**Goals:**
- One path for the size from scan to viewer, with the formatting in exactly one place.
- Catalogs and state databases written before this change keep working without a migration.

**Non-Goals:**
- Showing the size in tile captions or on the search page.
- Filtering or sorting by size.
- Re-reading file sizes from `/media/` at runtime, for example with HEAD requests.

## Decisions

### Carry the size as a required `size: u64` internally and an optional `size_bytes` in JSON
`CatalogItemInput` gets a required `size: u64`, because every scanned or stored row has one. `ItemView` gets `size: Option<u64>`, because a view rebuilt from an older shard may lack it. `ShardItem` gets `size_bytes: Option<u64>` with `#[serde(default, skip_serializing_if = "Option::is_none")]`, matching how `model` is handled.

The JSON name says its unit (`size_bytes`), like `duration_ms` does. Fresh output always writes the field, so new snapshots show it.

*Alternative:* bump `JSON_SCHEMA_VERSION` to 3. Rejected: the field is purely additive. The parser already rejects mismatched versions, so a bump would make the server refuse every catalog published before the upgrade until it is regenerated, for no benefit.

### Pass the size to the viewer through a `data-size` attribute
`TileView` gets a `size` field. `templates/tiles.html` emits `data-size="{{ size }}"` only when the size is known, the same way `data-width` works. `assets/search.js` sets `link.dataset.size` from `item.size_bytes` when it is a number. In `tileFromElement`, the viewer parses the attribute as a non-negative integer, or `null` when it is missing or invalid.

*Alternative:* render a hidden, pre-formatted label in the tile. Rejected: that puts the formatting in both Rust and JS, while a raw byte count keeps it in `viewer.js` only.

### Format sizes in a small exported helper `formatFileSize(bytes)`
Units are decimal (1 kB = 1000 B), matching the lowercase `kB` the user asked for and what macOS Finder shows.
- below 1000 B: `N B`
- below 1 MB: round to whole kB, `N kB`
- otherwise: one decimal in `MB` or `GB`

If rounding reaches 1000 of a unit, the helper moves up to the next unit, so 999,950 bytes shows `1.0 MB`, not `1000 kB`. The largest unit is `GB`. `captionText` then joins the timestamp, model, and size with ` · `, skipping any part that is empty. The helper goes on `__mediaCatalogViewer` so `tests/js/viewer_check.mjs` can test it directly.

*Alternative:* binary units (KiB/MiB). Rejected: the user asked for MB/kB.

### Leave the state database unchanged
`items.size` already exists and is refreshed on every scan (`upsert_scanned`), so `SCHEMA_VERSION` stays at 2.

## Risks / Trade-offs

- [Snapshot churn: every shard snapshot gains `size_bytes`] → Review the updated insta snapshots once. The diff should only add lines.
- [Many test fixtures build `CatalogItemInput` literals (render, json, events, server tests) and must gain `size`] → Mechanical edits. A missing field is a compile error, so nothing can be silently left out.
- [A server running an older catalog shows no sizes until the next generator run] → Expected and covered by the "Caption without a file size" scenario. A normal incremental run republishes shards with sizes.
- [The size is the file on disk at scan time; a file replaced since then shows its old size] → Same staleness as every other catalog field; the next scan fixes it.

## Migration Plan

Deploy the new binary and run the generator once. Shards then carry `size_bytes`. To roll back, run the previous binary; it ignores the unknown field, since `ShardItem` doesn't deny unknown fields.
