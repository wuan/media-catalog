# Tasks

## 1. Tile contract and page hooks

- [x] 1.1 Extend `TileView` and `templates/tiles.html` with the viewer metadata attributes (relative path, kind, capture label, thumbnail URL, dimensions when known), and verify rendered month and event pages contain them in `cargo test` snapshot updates.
- [x] 1.2 Add `defer` viewer-script tags to `templates/month.html`, `templates/event.html`, and `templates/search.html`, and verify rendered HTML references `/assets/viewer.js` exactly once per page.
- [x] 1.3 Update `assets/search.js` so dynamically built tiles emit the same data attributes, and verify the existing `tests/js/filter_check.mjs` contract still passes.
- [x] 1.4 Update render and server test snapshots for the new tile markup and script tags, and verify `cargo test` passes after snapshot review.

## 2. Asset delivery and styling

- [x] 2.1 Add `assets/viewer.js` with the viewer lifecycle skeleton and extend `assets/site.css` with dialog, stage, controls, caption, and reduced-motion styles; verify the styles load on a month page in a browser.
- [x] 2.2 Register and embed the viewer asset in `src/server/assets.rs` and the router; verify `GET /assets/viewer.js` returns the correct content type and ETag, and `HEAD` returns headers without a body.
- [x] 2.3 Add server tests covering viewer-asset serving before and after catalog publication, and verify `cargo test` passes.
- [x] 2.4 Document viewer controls, deep links, video focus behavior, and the no-JavaScript fallback in `README.md`; verify the documented behavior matches the shipped pages.

## 3. Album and viewer lifecycle

- [x] 3.1 Implement album extraction from the current page's tile links in document order and open a modal viewer on plain tile activation; verify month and event pages open the clicked item and modifier activation still opens the original.
- [x] 3.2 Implement close behavior via close control, backdrop, and Escape with focus restoration; verify focus returns to the originating tile and the page is interactive again.
- [x] 3.3 Implement `?item=` parsing and `replaceState` URL updates, preserving search filter parameters; verify refresh and shared links reopen the same item, stale paths leave the grid visible, and browser Back does not step through images.
- [x] 3.4 Implement previous/next controls, Left/Right keyboard navigation, and boundary stopping; verify first/last items disable the matching direction and arrows do not wrap.
- [x] 3.5 Add pure helper tests for item-parameter parsing, URL construction, album indexing, and boundary behavior; verify the JavaScript checks run under the existing Node test harness.

## 4. Image presentation and pan-zoom

- [x] 4.1 Render images with the thumbnail placeholder and original source, fit-to-screen on every item change, and reset zoom and pan between items; verify switching items never carries zoom forward.
- [x] 4.2 Implement Pointer Events pinch, one-pointer pan, clamped bounds, and horizontal swipe navigation at fit; verify touch and trackpad behavior on an image album.
- [x] 4.3 Implement wheel zoom, double-tap/double-click toggle, and `+`/`-`/`0` keyboard zoom with conservative limits; verify gestures zoom around the pointer and reset returns to fit.
- [x] 4.4 Implement adjacent-image preloading with data-saver awareness and no video preloading; verify only the current and neighbouring images are requested during navigation.
- [x] 4.5 Add unit tests for zoom clamping, transform bounds, and fit-to-swipe thresholds, and verify they pass in the JavaScript harness.

## 5. Inline video

- [x] 5.1 Render video items with native controls, thumbnail poster, `playsinline`, and metadata-only preload; verify a video opens paused and does not autoplay.
- [x] 5.2 Pause and rewind playback when navigating away or closing; verify audio never continues behind the viewer.
- [x] 5.3 Apply the focus rule so arrows navigate unless the video owns focus, and verify keyboard seeking works only while the player is focused.
- [x] 5.4 Provide a caption and direct original link when playback fails; verify an unsupported codec leaves a usable fallback.
- [x] 5.5 Add a browser verification checklist for video playback, pause-on-leave, and keyboard focus transitions.

## 6. Search integration and accessibility

- [x] 6.1 Emit a results-rendered event from `search.js` and have the viewer attach to the current filtered tiles and honor search deep links; verify filtering updates the album and an item removed by filters closes the viewer.
- [x] 6.2 Add modal dialog semantics, labelled controls, focus containment, position announcements, and accessible action names; verify keyboard-only navigation and screen-reader labels.
- [x] 6.3 Verify reduced-motion behavior, backdrop contrast, and mobile viewport layout on month, event, and search pages.
- [x] 6.4 Run the full integration suite: `cargo test`, `cargo fmt --check`, `cargo clippy -- -D warnings`, and the JavaScript harness; verify all pass.

## Workflow follow-up

- Archive the change after review requirements are satisfied.
- Verify the archived specs and change output.
