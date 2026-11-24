# Design

## Context

See `proposal.md` for motivation and `specs/media-viewer/spec.md` for the required behavior.

`assets/viewer.js` currently renders an image item by assigning the thumbnail URL to a visible `img.viewer-image` and, after `ORIGINAL_DELAY_MS`, loading the original into a detached `new Image()`. When that detached image fires `load`, the visible element gets the original URL:

```js
image.classList.remove("is-placeholder");
image.style.width = "";
image.style.height = "";
image.src = item.mediaUrl;
image.classList.add("is-original");
```

Chromium clears the visible element's bitmap when `src` changes and paints the newly sized box before the asynchronous decode finishes. Frame capture against the current code showed: thumbnail at ~83 ms, black stage at ~308 ms, original at ~391 ms. Firefox keeps the previous raster during the swap, which matches the reported browser difference.

The detached loader's `load` event only means the bytes are available; it does not guarantee a decoded bitmap for the visible element. Decoded bitmaps are not reliably shared across elements.

## Goals / Non-Goals

**Goals:**

- Keep the thumbnail on screen until the original can be painted.
- Make the thumbnail-to-original transition one paint in Chromium.
- Preserve the existing deferred fetch, pan/zoom state, slideshow, and error behavior.
- Add a deterministic browser regression that fails the current src-swap implementation.

**Non-Goals:**

- Changing placeholder geometry, thumbnail generation, or fitted-box sizing.
- Adding a fade or animation between preview and original.
- Changing the 200 ms delay or resolving the spec's 300 ms wording (separate cleanup).

## Decisions

### Decision: Decode the original on the detached loader, then adopt that element

When the detached original fires `load`, call `original.decode()`. Once it resolves, style the loader element as `img.viewer-image.is-original`, insert it in place of the thumbnail element, and update `state.media` to it.

A decoded element has paint-ready content when it enters the document, so Chromium has no empty frame to paint. Verified with a 6000x4500 JPEG and with a 12000x9000 JPEG: no black compositor frame was captured; thumbnail frames were followed directly by original frames.

The thumbnail element is removed only in the same DOM operation that inserts the decoded original. The stage therefore always holds either the thumbnail or a decoded original.

**Alternatives considered:**

- `image.decoding = "sync"` before the `src` swap. Verified to remove the black frame, but it forces a synchronous decode at swap time on the main thread; large originals can stall input and animation. It also relies on a browser hint.
- Keep both images layered and fade the original in. Visually smoother but requires absolute layering, duplicated transform/crop handling, and more CSS. The spec does not require a transition, so this is deferred.

### Decision: Guard adoption with the existing item identity

The decode promise resolves asynchronously. Adoption SHALL proceed only when `state.media` still points at the thumbnail element it replaced. Navigating, closing, or a slideshow advance during decode therefore leaves the new item's preview in place and discards the stale original. `cancelOriginal()` continues to drop the loader's source and handlers when an item is left before decode completes.

### Decision: Decode failure keeps the preview

If decode rejects, the viewer does not adopt the element and the thumbnail remains visible. A load error already follows the same path today. This satisfies the "original cannot be loaded or decoded" scenario without introducing a blank stage.

If `HTMLImageElement.decode` is unavailable, set `decoding = "sync"` on the loader element before adoption as a fallback; the primary path assumes `decode()` support, which current Chromium, Firefox, and Safari provide.

### Decision: Re-apply fit and transform to the adopted element

`measureFit()` and `applyTransform()` currently operate on `state.media`. After adoption, set `state.media` to the decoded element and run `measureFit()` followed by `applyTransform()`. This keeps the current zoom/pan transform on the element that is now rendered and recomputes fit bases from the original's natural size when the catalog record has no dimensions.

The adopted element keeps the thumbnail's `alt`, `draggable`, and decoding-related behavior, and uses the same `viewer-image` class. `is-original` remains a marker only; it has no CSS rules today.

### Decision: Test the decode gate in the browser regression

Extend `tests/js/viewer_browser.mjs` with a scenario that serves an original for `/media/fixture/landscape.jpg` and installs an init script that delays `HTMLImageElement.prototype.decode` for that URL. After the original's `load` fires but before decode finishes, the test asserts that `.viewer-image` still has `is-placeholder` and still points at the thumbnail. It then waits for `.viewer-image.is-original` and asserts the adopted element shows the original.

This is deterministic, needs no pixel decoding, and fails the current src-swap code because that code removes the placeholder immediately and never calls decode. A frame-capture pixel assertion remains possible later but is not required for this change.

## Risks / Trade-offs

- [Brief coexistence of the thumbnail bitmap and the decoded original] → Both are held only until the DOM replacement; the thumbnail element is then released.
- [Large decode delays keep the thumbnail visible longer than today] → Intended: the preview stays until the original is paintable, and the spec's "shown immediately" behavior is unchanged.
- [decode rejection leaves the original unpainted] → The preview remains usable and the item caption and original link stay available, matching the existing failure path.
- [Tests patching `decode` could mask unrelated decode behavior] → The override is scoped to the fixture's original URL and only delays resolution; the test also asserts the adopted original is present afterward.
- [Zoom applied while the thumbnail is visible uses thumbnail dimensions when catalog dimensions are missing] → Adoption re-runs `measureFit()` from the original's natural size and re-applies the current transform, correcting the fit bases.

## Migration Plan

Frontend asset change only. Publish the updated `assets/viewer.js` with the next catalog publish; rollback is redeploying the previous assets. The browser regression skips when Node or Chromium is unavailable, as the existing test does.
