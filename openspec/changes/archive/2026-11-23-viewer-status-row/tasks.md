# Tasks

## 1. Restructure the viewer status

- [x] 1.1 In `assets/viewer.js`, create a `.viewer-status` wrapper inside `.viewer-caption` and append the captured, location, and meta segments in left-to-right order; verify via the browser fixture that the rendered DOM order is captured, location, meta under `.viewer-status`
- [x] 1.2 Keep `aria-labelledby` on `viewer-captured` and `aria-describedby` on `viewer-caption`, and keep `hidden` toggling for optional segments; verify the dialog's accessible name still contains only the capture caption and its description includes settings and location
- [x] 1.3 In `assets/site.css`, replace the per-segment top margins with a three-column `.viewer-status` grid: caption left, location centred, settings right; verify computed positions in the browser fixture that caption starts at the row's left edge, location's centre matches the row's centre, and settings end at the row's right edge
- [x] 1.4 Add the narrow-viewport rule that stacks `.viewer-status` into one column, with caption, coordinates, and settings in reading order; verify at a 375 px viewport that the row does not overflow horizontally or overlap text
- [x] 1.5 Add browser regression assertions in `tests/js/viewer_browser.mjs` for the status row's geometry and single-row rendering; run `npm run test:viewer` and verify it passes
- [x] 1.6 Update the viewer caption, camera settings, and location bullets in `README.md` to describe the single status row and its left/centre/right layout; verify the documented example matches the rendered text

## 2. Cover optional segments

- [x] 2.1 Extend `tests/js/fixtures/viewer-browser.html` with an item that has no coordinates or camera settings, and update the tile-count assertion; verify the status row keeps the caption at the left and renders no empty centre or right segments
- [x] 2.2 Add regression coverage for an item with settings but no coordinates and an item with coordinates but no settings, so the remaining segments keep their assigned positions; verify `npm run test:viewer` passes
- [x] 2.3 Add regression assertions that the location link remains centred, labelled with the coordinates and OpenStreetMap, and that no map request occurs before activation; verify those assertions pass in the browser regression

## 3. Integration verification

- [x] 3.1 Run `cargo test --test viewer` and verify both viewer tests pass
- [x] 3.2 Run `npm run test:viewer` once more against the final assets and verify it reports the browser regression as passed

## Workflow follow-up

- Archive the change after the project's review requirements are satisfied.
