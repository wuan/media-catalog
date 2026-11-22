# Tasks

## 1. Probe model metadata

- [x] 1.1 Add a model field to `ExifMetadata` and `ProbeResult`; read IFD0 tag `0x0110`, trim and collapse whitespace, and treat empty values as absent. Verify with unit tests covering a model-bearing TIFF, an empty value, and a missing tag.
- [x] 1.2 Read video model tags (`com.apple.quicktime.model`, then `model`) from ffprobe format and stream tags and populate `ProbeResult.model`. Verify with unit tests over synthetic ffprobe JSON with and without a model tag.
- [x] 1.3 Extend probe tests to confirm model extraction does not expose GPS or serial data and that normalized model text stays on one line. Verify `cargo test probe` passes.

## 2. State schema and persistence

- [x] 2.1 Add `model` to `ItemRecord`, the `SELECT`, `set_probe`, and upsert paths. Verify a state round-trip test reads back a stored model.
- [x] 2.2 Bump `SCHEMA_VERSION` to 2 and migrate v1 databases by adding `model TEXT` and clearing cached probe fields once so unchanged rows re-probe without regenerating thumbnails. Verify a test opens a v1-shaped database, migrates it, and sees null probe metadata plus the new column.
- [x] 2.3 Confirm migrated rows are probed again once and unchanged fully probed rows are skipped afterward. Verify with a focused state or pipeline test.

## 3. Catalog model and JSON contract

- [x] 3.1 Add `model` to `CatalogItemInput`, `ItemView`, and `ShardItem`; serialize the shard field only when known. Verify catalog construction and JSON round-trip tests preserve the model.
- [x] 3.2 Bump `JSON_SCHEMA_VERSION` to 2 and update JSON snapshots to include `model` where present. Verify `cargo test json` passes and snapshots are accepted.
- [x] 3.3 Update the privacy test to allow `model` while rejecting GPS, location, latitude/longitude, and serial fields. Verify the test fails if a forbidden field is reintroduced.

## 4. Tile rendering

- [x] 4.1 Give `TileView` a short caption timestamp (`%y-%m-%d %H:%M`) separate from the raw capture timestamp used by `data-captured`. Verify a render unit test asserts `24-05-01 10:00` while the attribute keeps `2024-05-01 10:00:00`.
- [x] 4.2 Update `templates/tiles.html` to render a `<time>` element plus an optional model span while keeping the raw timestamp in `data-captured`. Verify event and month snapshots show `24-05-01 10:00 iPhone 15 Pro` and the date-only fallback.
- [x] 4.3 Update `assets/site.css` so `figcaption` stays on one non-wrapping line with ellipsis overflow. Verify a rendered month page at narrow width keeps the caption on one line.
- [x] 4.4 Keep filename alternative text while removing the visible `.name` caption span. Verify snapshots retain `alt="a.jpg"` and the accessibility render test passes.

## 5. Search grid caption

- [x] 5.1 Update `assets/search.js` `buildTile` to format `captured_at` as `YY-MM-DD HH:MM`, append the model when present, and keep the raw value in `data-captured`. Verify the JavaScript filter check asserts captions with and without a model.
- [x] 5.2 Remove the `.name` span from search tiles while keeping filename-derived alternative text. Verify the viewer check still opens results and derives the filename title from `data-item-path`.

## 6. Integration checks

- [x] 6.1 Run `cargo test` and update expected snapshots; verify all Rust and server tests pass.
- [x] 6.2 Run the JavaScript checks and confirm month, event, and search grids render the same one-line caption format. Verify `tests/js/filter_check.mjs` and `tests/js/viewer_check.mjs` pass.
- [x] 6.3 Run `openspec validate concise-tile-captions --strict` and verify the change validates.

## 7. Documentation

- [x] 7.1 Update `README.md` to describe the one-line tile caption, the model-privacy boundary, and the rebuild/migration requirement. Verify the documented behavior matches the rendered pages.

## Workflow follow-up

- Archive the change after review.
- Verify the archived specs reflect the model privacy and caption behavior.
