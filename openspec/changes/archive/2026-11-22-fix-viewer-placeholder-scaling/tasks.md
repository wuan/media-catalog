# Tasks

## 1. Implement placeholder sizing helper

- [x] 1.1 Add `placeholderBox(width, height, stageWidth, stageHeight)` to `assets/viewer.js` that returns square dimensions matching the longer edge of the fitted box; verify with `node tests/js/viewer_check.mjs assets/viewer.js` printing correct `placeholderWide`/`placeholderTall` values.
- [x] 1.2 Export `placeholderBox` on `__mediaCatalogViewer` in `assets/viewer.js`; verify it appears in the test script output.

## 2. Update renderImage to use placeholderBox

- [x] 2.1 In `renderImage` (`assets/viewer.js`), replace the `fittedBox` call for the placeholder with `placeholderBox` so the placeholder fills the longer edge of the original's fitted box; verify by loading a test page in a browser and confirming the placeholder matches the original's longer-edge area before swap.
- [x] 2.2 Keep `fittedBox` available for `measureFit` pan/zoom bounds; verify `cargo test --test viewer` still passes.

## 3. Update tests

- [x] 3.1 Add `placeholderWide` and `placeholderTall` assertions to `tests/js/viewer_check.mjs`; verify `node tests/js/viewer_check.mjs assets/viewer.js` outputs the expected square dimensions.
- [x] 3.2 Update `tests/viewer.rs` with assertions for `placeholderWide` and `placeholderTall`; verify `cargo test --test viewer` passes.

## 4. Integration verification

- [x] 4.1 Run the full test suite with `cargo test` and verify all tests pass.
