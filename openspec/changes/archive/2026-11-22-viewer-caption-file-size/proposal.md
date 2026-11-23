# Proposal

## Why

The viewer caption shows when an item was taken and with which camera, but not how large the original is. A visitor deciding whether to open or download an original, especially a long video over a slow link, has no hint of its size. The scanner already records each file's byte size, but the published catalog doesn't include it yet.

## What Changes

- The viewer caption line gains the original's file size after the timestamp and camera model, for example `2024-05-01 10:11:12 · iPhone 15 Pro · 3.4 MB`. Sizes use decimal units (`B`, `kB`, `MB`, `GB`).
- Year shard item records gain an optional `size_bytes` field with the original's byte size. It is additive: catalogs published before this change still load, and their items show no size in the viewer.
- Rendered tiles (server pages and search results) carry the byte size so the viewer can read it from the tile, just as it does for the timestamp and model. Tile captions themselves don't change.
- The size applies to every item the viewer opens, videos included, because images and videos share one caption line.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `media-viewer`: the "Caption the open item" requirement adds the file size to the caption line.
- `static-catalog`: the "Emit sharded JSON catalog data" requirement adds the optional byte size to item records.
- `media-inventory`: the "Extract safe capture metadata" requirement lists the file size as published metadata, so the privacy rule stays accurate.

## Impact

- `src/model.rs`, `src/lib.rs`: carry the scanned byte size from scan and state records into the catalog item view.
- `src/json.rs`: serialize and parse `size_bytes`. No JSON schema version bump, because the field is optional and defaults to absent.
- `src/render.rs`, `templates/tiles.html`, `assets/search.js`: emit a `data-size` attribute on tile links.
- `assets/viewer.js`: read the size from the tile, format it, and append it to the caption.
- Tests: JSON snapshots, render tests, and the viewer JS check. Test fixtures that build catalog input rows gain the new field.
- `README.md`: the metadata list, the shard example, and the viewer section.
- No state database migration: `items.size` already exists.
