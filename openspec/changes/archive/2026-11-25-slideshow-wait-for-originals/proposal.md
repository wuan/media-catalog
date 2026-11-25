## Why

In slideshow playback the viewer currently paints each item's thumbnail and advances on a fixed timer regardless of whether the original has arrived. That shows a low-resolution crop where a full image is expected, and a slow load can produce a slide that is only partly visible before it disappears.

## What Changes

- Slideshow playback never uses the thumbnail preview: it requests each slide's original immediately and keeps the current image on screen until the next original is downloaded **and decoded**.
- The 5-second dwell starts after the next image paints, not when navigation begins.
- The next image item's original is prefetched as soon as the current image paints, reusing the downloaded element so the swap is instant when possible.
- Video items are skipped during slideshow playback instead of pausing the slideshow. They remain reachable by manual navigation.
- Failed original loads are retried a limited number of times, then the slideshow stops and opens the failed item in the normal paused viewer with its thumbnail preview, caption, and an error note.
- Manual navigation keeps the existing immediate-thumbnail preview behavior.

## Capabilities

### New Capabilities

<!-- None: this change modifies existing viewer behavior. -->

### Modified Capabilities

- `media-viewer`: slideshow playback no longer shows thumbnails or advances on a fixed timer; it waits for decoded originals, prefetches the next image, skips videos, and has a bounded retry/failure path.

## Impact

- `assets/viewer.js`: slideshow target selection, advance/prefetch flow, original loading and retries, failure state, URL/chrome commit timing.
- `assets/site.css`: only if a failure note needs styling beyond the existing `.viewer-error`.
- `tests/js/viewer_browser.mjs`, `tests/js/viewer_check.mjs`, `tests/viewer.rs`: new slideshow loading, skip, prefetch, retry, and failure coverage.
- `README.md`: slideshow behavior description.
