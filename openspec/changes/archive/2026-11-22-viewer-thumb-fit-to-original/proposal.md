# Proposal

## Why

The in-page viewer shows the 512x512 square thumbnail while the full-resolution
image loads, then swaps in the original. Because the thumbnail is a center
square crop but the original keeps its real aspect ratio, the displayed image
box changes size when the original arrives, causing a visible layout jump.
Sizing the thumbnail placeholder to the fitted box of the real image makes the
original appear exactly on top of it.

## What Changes

- The viewer SHALL display the thumbnail placeholder for an image item at the
  same fitted size and position the full-resolution original will occupy, using
  the item's upright width/height already exposed as `data-width`/`data-height`
  on the tile. The 512x512 thumbnail content is cover-scaled into that box, so
  it appears lower-resolution and center-cropped until the original loads.
- When width/height are unavailable, the viewer keeps the current behavior of
  fitting the thumbnail's own 512x512 dimensions.
- Video items are out of scope; the video poster display is unchanged.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `media-viewer`: the "Present images with pan and zoom" requirement gains a
  no-layout-shift condition for the thumbnail-to-original swap.

## Impact

- `assets/viewer.js`: `renderImage` and `measureFit` use the item's real
  dimensions to size the thumbnail placeholder and compute pan/zoom bounds.
- `assets/site.css`: thumbnail placeholder uses `object-fit: cover` within the
  real-image box while the original is loading.
- `tests/js/viewer_check.mjs` and `tests/viewer.rs`: cover the new sizing rule.
- No changes to thumbnail generation, templates, the server, or stored data.
