# Tasks

## 1. Viewer placeholder sizing

- [x] 1.1 In `assets/viewer.js` `renderImage`, when showing the thumbnail and the item has `width`/`height`, set the image element's explicit fitted size from the real dimensions with `object-fit: cover`; verify by opening a 4000x3000 item in the viewer and confirming the placeholder box matches the original's box after swap (no layout shift)
- [x] 1.2 In `assets/viewer.js` `measureFit`, prefer `item.width`/`item.height` over the thumbnail's natural 512x512 when the current item provides them, so pan/zoom clamp bounds use the original's fitted size; verify zooming while the placeholder is visible clamps against the real-image box
- [x] 1.3 Clear or match the explicit placeholder sizing when the original finishes loading so the swap keeps the identical box; verify no visible resize occurs in the browser

## 2. Styling

- [x] 2.1 Add a CSS rule in `assets/site.css` for the loading placeholder state (cover fit within the real-image box); verify the rule applies only while the thumbnail is displayed

## 3. Tests

- [x] 3.1 Extend `tests/js/viewer_check.mjs` to expose the new sizing helper(s) and assert the fitted box is computed from `data-width`/`data-height`; verify with `cargo test --test viewer`
- [x] 3.2 Add assertions in `tests/viewer.rs` for the new helper results; verify with `cargo test --test viewer`

## 4. Integration check

- [x] 4.1 Run the full test suite (`cargo test`) and confirm all existing viewer, thumbnail, and server tests still pass
