# Tasks

## 1. Caption formatting

- [x] 1.1 Add a megapixel formatter in `assets/viewer.js` that turns pixel width and height into the one-decimal `MP` value used by the caption, and export it through `__mediaCatalogViewer` so the Node harness can exercise it; verify `node tests/js/viewer_check.mjs assets/viewer.js` still parses and prints the harness JSON
- [x] 1.2 Insert the `<width>x<height> <megapixels>MP` group into `captionText` between the camera model and the file size whenever both dimensions are known, and omit the group and its separator otherwise; verify the harness output shows `4000x3000 12.0MP` in the sized caption, the same group for a video fixture, and no dimension text for an item with a missing dimension

## 2. Regression tests

- [x] 2.1 Extend `tests/js/viewer_check.mjs` and `tests/viewer.rs` with caption expectations for an image with dimensions, a video with dimensions, and an item missing one dimension, covering `12.0MP`, `2.1MP`, and a rounded case such as `12.2MP`; verify `cargo test --test viewer` passes
- [x] 2.2 Update the expected caption in `tests/js/viewer_browser.mjs` to include `4000x3000 12.0MP` before `3.4 MB`, and verify `npm run test:viewer` passes with Playwright available

## 3. Integration check

- [x] 3.1 Run the full Rust test suite with `cargo test` and confirm the viewer caption change does not disturb render, server, or snapshot tests
- [x] 3.2 Run `openspec validate add-viewer-image-dimensions --strict` and confirm the change and its delta spec report valid

## Workflow follow-up

- Archive the change after the project's review requirements are satisfied, then verify the archived `media-viewer` spec contains the updated caption requirement.
