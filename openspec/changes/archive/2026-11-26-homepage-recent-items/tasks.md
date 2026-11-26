# Tasks

## 1. Recent-item selection in the renderer

- [x] 1.1 Add the recent tile list to `IndexTemplate` and select at most 24 items from `&Catalog`, ordered by capture timestamp descending then relative path descending; verify with a unit test that mixed folder/capture timestamps and a tie produce the expected newest-first paths.
- [x] 1.2 Cover the selection edges with unit tests: a catalog smaller than 24 returns every item, and an empty catalog returns an empty list; verify `cargo test` passes.
- [x] 1.3 Convert the selected `ItemView`s with the existing `tile_from_item` path and verify a unit test shows videos keep their video marker and missing thumbnails stay placeholders.

## 2. Root page template and styles

- [x] 2.1 Add the third `Recent` section after Years in `templates/index.html`, include `tiles.html`, load `viewer.js`, and emit the location-preview body attribute from the catalog; verify `render_index` snapshots contain the heading, tile links, and script tag.
- [x] 2.2 Update the affected root-page insta snapshots (`index_empty`, `index_single_year`, `index_multi_year`) and verify `cargo test` passes with empty catalogs omitting the section.
- [x] 2.3 Add a `.recent-grid` rule with six columns at desktop width and three/two-column breakpoints for narrower viewports, without clipping or hiding tiles; verify in a browser at wide, medium, and narrow widths that 24 items occupy four, eight, and twelve rows respectively.
- [x] 2.4 Document the recent section in the `README.md` root route list and the `docs/server.md` route table; verify each description matches the rendered page.

## 3. Root-page integration and viewer behavior

- [x] 3.1 Extend `tests/end_to_end.rs` root assertions: the recent section follows Years, its tiles are newest-first across folders, videos are included, and the viewer script is loaded; verify `cargo test --test end_to_end` passes.
- [x] 3.2 Add an end-to-end case with an empty published catalog and verify `/` omits the `Recent` heading while still rendering years content.
- [x] 3.3 Extend the viewer browser regression with a root fixture whose recent tiles span two months; verify opening a tile and using previous/next stays within the recent tiles in newest-first order, and that tiles still link to originals when scripting is disabled.

## 4. Final checks

- [x] 4.1 Run `cargo test`, `cargo fmt --check`, and `cargo clippy -- -D warnings`; verify all pass.
- [x] 4.2 Run the server against a sample catalog and confirm `/` renders four desktop rows, reflows without clipping on a narrow window, opens the viewer from a recent tile, and keeps tile links working without JavaScript.

## Workflow follow-up

- Archive the change after review requirements are satisfied.
