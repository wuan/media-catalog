# Tasks

## 1. Catalog model and JSON

- [x] 1.1 Add a required `size: u64` to `CatalogItemInput` and `size: Option<u64>` to `ItemView` in `src/model.rs`, and fill it in `build_catalog`. Set it from `file.size` / `record.size` in both input builders in `src/lib.rs`. Add the field to every `CatalogItemInput` literal in the `src/` test modules and `tests/`. Verify `cargo build --all-targets` compiles.
- [x] 1.2 Add `size_bytes: Option<u64>` (`serde(default, skip_serializing_if = "Option::is_none")`) to `ShardItem` in `src/json.rs` and map it in `From<&ItemView>` and `to_item_view`. Add a test that a shard item with no `size_bytes` parses to `size: None`, and a test that a written shard contains `"size_bytes"`. Verify `cargo test json` passes.
- [x] 1.3 Review and accept the updated JSON snapshots (`cargo insta review` or `INSTA_UPDATE=always`), confirming the diff only adds `size_bytes` lines. Verify `cargo test` passes with no pending snapshots.
- [x] 1.4 Update `README.md`: add file size to the "Metadata and privacy" list and its published-fields paragraph, add `"size_bytes"` to the shard example, and note that the field is optional for older catalogs. Verify by reading the rendered sections.

## 2. Tile attributes

- [x] 2.1 Add `size: Option<u64>` to `TileView` in `src/render.rs` and emit `data-size="…"` in `templates/tiles.html` only when the size is known. Add a render test asserting `data-size="3400000"` on a tile with a size and no `data-size` on a tile without one. Verify `cargo test render` passes and any changed HTML snapshots only add `data-size`.
- [x] 2.2 In `assets/search.js` `buildTile`, set `link.dataset.size` when `item.size_bytes` is a number. Verify by loading `/search.html` against a regenerated catalog and checking that a result tile has `data-size` in devtools.

## 3. Viewer caption

- [x] 3.1 In `assets/viewer.js`, add `formatFileSize(bytes)` (decimal units, `B`/`kB` as whole numbers, `MB`/`GB` with one decimal, moving up a unit when rounding reaches 1000) and export it on `__mediaCatalogViewer`. Extend `tests/js/viewer_check.mjs` and `tests/viewer.rs` with 845 → `845 B`, 532000 → `532 kB`, 999950 → `1.0 MB`, 3400000 → `3.4 MB`, 1240000000 → `1.2 GB`, and null → `""`. Verify `cargo test --test viewer` passes.
- [x] 3.2 Read `data-size` in `tileFromElement` (non-negative integer, else `null`) and make `captionText` join timestamp, model, and formatted size with ` · `, skipping empty parts. Extend the viewer JS check for a tile with model and size, one with size only, and one with neither. Verify `cargo test --test viewer` passes.
- [x] 3.3 Update the `README.md` "In-page media viewer" section to say the caption shows the timestamp, the camera model when known, and the file size. Verify by reading the section.

## 4. Integration check

- [x] 4.1 Run `cargo test` and `openspec validate viewer-caption-file-size --strict`, both passing. Then run the generator on a small fixture tree, start the server, open an image and a video in the viewer from a month page and from search, and confirm the caption reads `<timestamp> · <model> · <size>`, or `<timestamp> · <size>` when there's no model.

## Workflow follow-up

- Archive the change once implementation is reviewed, syncing the delta specs into `openspec/specs/`.
