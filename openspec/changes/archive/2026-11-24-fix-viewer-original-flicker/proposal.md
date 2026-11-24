# Proposal

## Why

In Chromium the viewer paints a short black frame between the thumbnail preview and the original image: changing `src` on the visible `<img>` clears the current bitmap while the original is still decoding. Firefox keeps the previous frame visible, so the defect only shows on Chromium-based browsers. Every image open goes through this swap, so the flicker is visible throughout the catalog.

## What Changes

- Replace the visible-image `src` swap with a decode-then-adopt path: keep the thumbnail element on screen, load the original into a detached image, wait for `decode()`, then adopt the decoded element as the viewer media.
- Keep the thumbnail preview visible until the decoded original is ready, and guard adoption so navigation, slideshow advances, or closing during decode cannot install a stale item.
- Re-apply the active fit/transform and re-measure fit when the adopted element becomes the viewer media.
- Add a browser regression that serves a real original and verifies the thumbnail stays painted until the original is shown, instead of asserting geometry only while originals are unavailable.
- Add a spec scenario requiring that the replacement never paints a frame showing neither the thumbnail nor the original.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `media-viewer`: the "Present images with pan and zoom" requirement gains a behavior for the thumbnail-to-original replacement: the viewer SHALL keep the thumbnail visible until the original can be painted and SHALL NOT paint an empty stage between them.

## Impact

- `assets/viewer.js`: `loadOriginal`, `renderImage`, and the shared `state.media` handling.
- `tests/js/viewer_browser.mjs` and `tests/js/fixtures/viewer-browser.html`: browser fixture must serve originals and observe the swap.
- `openspec/specs/media-viewer/spec.md`: one added scenario under image presentation.
- No CLI, catalog schema, or server changes.
