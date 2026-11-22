# Proposal

## Why

The 512x512 center-crop thumbnail placeholder shown while the full-resolution original loads is scaled incorrectly. The placeholder should be sized so it fills the longer edge of the original image's fitted box, matching what the original will occupy. Currently it appears scaled to the shorter edge, producing a mismatched preview that does not represent the final display size.

## What Changes

- The placeholder sizing logic in `assets/viewer.js` (`renderImage`) will compute the placeholder dimensions using the longer edge of the fitted box rather than the contain-fit box.
- The `fittedBox` helper will be replaced or complemented by a helper that returns the cover-fit dimensions for the placeholder, so the square thumbnail fills the longer edge of the original's display area.
- The CSS `object-fit` behavior for `.viewer-image.is-placeholder` will be adjusted if needed to avoid double-cropping the square thumbnail.
- Tests in `tests/js/viewer_check.mjs` and `tests/viewer.rs` will be updated to cover the new sizing behavior.

## Capabilities

### Modified Capabilities
- `media-viewer`: The "Present images with pan and zoom" requirement's placeholder sizing scenario changes — the thumbnail placeholder SHALL be scaled to fill the longer edge of the original's fitted box instead of the contain-fit box.

## Impact

- `assets/viewer.js`: `renderImage` placeholder sizing and the `fittedBox` helper.
- `assets/site.css`: possibly `.viewer-image.is-placeholder` rules.
- `tests/js/viewer_check.mjs`: new/updated assertions for placeholder sizing.
- `tests/viewer.rs`: updated expected values for the JS helper outputs.
