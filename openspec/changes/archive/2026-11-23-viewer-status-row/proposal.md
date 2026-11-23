# Proposal

## Why

The viewer's status area currently stacks three lines: the capture caption, the camera settings, and the location coordinates. This consumes vertical space that could show more of the image, and it separates related metadata that a visitor reads together. A single status row keeps the image larger and makes the coordinates and camera settings easier to scan at a glance.

## What Changes

- Replace the viewer's three stacked caption lines with one status row.
- Keep the capture caption (timestamp, camera model, file size) at the left of the row.
- Place the location coordinates at the centre of the row, still linked to OpenStreetMap and still omitted when coordinates are unavailable.
- Place the camera settings (focal length, aperture, exposure, ISO) at the right of the row, still omitting unknown values.
- Keep each part independently optional: absent parts leave the remaining parts in their assigned positions without empty separators or controls.
- Let the row wrap or stack on narrow viewports so it never overflows or forces horizontal scrolling.
- Keep the capture caption as the dialog's accessible name, and keep the settings and location available through the dialog description and the labelled location link.
- Update the README viewer documentation and the viewer browser regression coverage to describe and verify the new layout.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `media-viewer`: replaces the separate caption, camera-settings, and location lines with one status row whose layout is caption left, coordinates centred, camera settings right, including responsive wrapping and independently optional parts.

## Impact

- `assets/viewer.js`: caption DOM construction and item rendering.
- `assets/site.css`: viewer caption layout and responsive styles.
- `tests/js/viewer_browser.mjs`: rendered layout assertions for the status row.
- `README.md`: viewer caption, camera settings, and location documentation.
- `openspec/specs/media-viewer/spec.md`: caption, camera settings, geocoordinate, and accessibility requirements.
