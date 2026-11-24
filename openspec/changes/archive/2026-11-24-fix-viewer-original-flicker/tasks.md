# Tasks

## 1. Decode-then-adopt image replacement

- [x] 1.1 Replace the visible-element `src` swap in `loadOriginal` with decode-gated adoption: wait for the detached original's `decode()`, then insert that decoded element in place of the thumbnail and update `state.media`; verify `node tests/js/viewer_check.mjs assets/viewer.js` still passes and an image opens without a blank stage in Chromium.
- [x] 1.2 Style the adopted element as `img.viewer-image.is-original` with the item's `alt` and `draggable` behavior, then re-run `measureFit()` and `applyTransform()` so fit bases and the active transform follow the new media element; verify `npm run test:viewer` keeps the existing geometry, immersive, and zoom assertions green.
- [x] 1.3 Keep the thumbnail visible when the original fails to load or decode, and guard adoption so navigating, closing, or a slideshow advance during decode does not install the stale original; verify the preview remains and the next item renders through the browser scenario.
- [x] 1.4 Update the "Images" bullet in the README's in-page media viewer section to state that the thumbnail stays visible until the original is decoded; verify the documented behavior matches the implementation and the README's test commands still run.

## 2. Browser regression for the replacement

- [x] 2.1 Serve a real original from the `tests/js/viewer_browser.mjs` route map for `/media/fixture/landscape.jpg` and install a scoped override that delays `HTMLImageElement.prototype.decode` for that URL; verify the fixture still loads and the existing geometry assertions pass.
- [x] 2.2 Add a scenario that, while decode is pending, asserts `.viewer-image` still has `is-placeholder` and still points at the thumbnail URL, then waits for `.viewer-image.is-original` and asserts the original is adopted; verify the scenario fails against the old src-swap code and passes against the decode-then-adopt code with `npm run test:viewer`.
- [x] 2.3 Add a navigation-during-decode assertion to the same scenario: open the next item while the original is decoding and assert the previous original is never installed; verify the new item's preview stays visible after `npm run test:viewer`.

## 3. Integration verification

- [x] 3.1 Run `cargo test --test viewer` and confirm the JS helper checks and the Chromium browser regression both pass or skip with the expected diagnostic.
- [x] 3.2 Run `openspec validate fix-viewer-original-flicker --strict` and confirm the proposal, spec delta, design, and tasks validate without warnings.
