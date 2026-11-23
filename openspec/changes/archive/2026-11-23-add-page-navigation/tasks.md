# Tasks

## 1. Month navigation

- [x] 1.1 Add optional previous/next month link fields to `MonthTemplate` and resolve the nearest existing month pages across year boundaries and gaps in `render_month`; verify new `src/render.rs` tests cover same-year adjacency, skipped gaps, cross-year navigation, earliest/latest, and the single-month case
- [x] 1.2 Render the month navigation above the heading and again after the tile grid using a shared `templates/month-nav.html` include with full month-and-year labels and `rel="prev"`/`rel="next"`; verify render assertions find both nav positions with the expected hrefs and labels
- [x] 1.3 Style the month navigation in `assets/site.css` so the two links sit at opposite edges and an absent side leaves the remaining link aligned; verify the rendered page carries the nav class at both positions and a first/latest month page renders only one link
- [x] 1.4 Update the `month_2024_05` snapshot and add a multi-year month navigation snapshot; verify `cargo test` passes and the snapshot diff contains only the intended navigation markup

## 2. Event back-to-top links

- [x] 2.1 Add a top anchor to the event page and render a `Back to top` link after the tile grid of the `Other` bucket, each authored section, and each month group on unsectioned events; verify render assertions find exactly one link per rendered group and that it follows the grid
- [x] 2.2 Style the back-to-top link in `assets/site.css` as a subordinate muted line; verify the rendered pages carry the link class and the existing section navigation remains unchanged
- [x] 2.3 Add render tests for authored sections, `Other`, month groups, and the empty event; verify each covered group has one `href="#top"` link, the empty event has none, and no JavaScript is required
- [x] 2.4 Update the `event_may_trip` and `event_portugal_sections` snapshots; verify `cargo test` passes and the snapshot diffs add only the top anchor and back-to-top links

## 3. Integration verification

- [x] 3.1 Run `cargo fmt --check`, `cargo clippy --locked --all-targets -- -D warnings`, and `cargo test --locked --all-targets`; verify all three pass
- [x] 3.2 Build and serve a catalog with cross-year months and a sectioned event; verify month previous/next links resolve, event back-to-top links return to the top, and both behave with JavaScript disabled
- [x] 3.3 Run `openspec validate add-page-navigation --strict`; verify the change validates without warnings

## Workflow follow-up

- Archive the change after the project's review requirements are satisfied.
