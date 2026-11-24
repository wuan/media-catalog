# Proposal

## Why

The viewer caption reports when a photo was taken, the camera model, and the
file size, but not the image's pixel dimensions. Visitors comparing shots (for
example, deciding whether a crop or a print is viable) have to open the original
to learn its resolution, even though the catalog already records upright
dimensions for every tile.

## What Changes

- Add the pixel dimensions and megapixel count to the viewer's capture caption,
  placed immediately before the file size: `4000x3000 12.0MP`. Image and video
  items both show the group when their dimensions are known.
- Megapixels always show one decimal place (`12.0MP`, `12.2MP`), computed from
  the orientation-corrected dimensions the catalog already stores.
- Keep the existing caption order: timestamp, then camera model, then dimensions,
  then file size.
- Omit the dimension group, and its separator, whenever either dimension is
  unknown, matching the caption's existing rule that unknown parts collapse
  without leaving empty separators.
- Leave the centred coordinates segment, the right camera-settings segment, and
  the dialog's accessible name behaviour unchanged. Because the caption segment
  is already the dialog's accessible name, the dimensions are announced without
  additional accessibility plumbing.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `media-viewer`: the caption requirement now includes the original's pixel
  dimensions and megapixel count between the camera model and the file size.

## Impact

- `assets/viewer.js`: caption text assembly and the exported helper surface used
  by the Node test harness.
- `tests/js/viewer_check.mjs` and `tests/viewer.rs`: caption expectations.
- `openspec/specs/media-viewer/spec.md`: caption requirement and scenarios.
- No server, render, probe, or template changes: tiles already expose
  `data-width`, `data-height`, and `data-size`, and stored dimensions are already
  orientation-corrected.
