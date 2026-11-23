# Proposal

## Why

The viewer's thumbnail preview is scaled from the stage size measured before the
caption and metadata chrome is populated, and it is never resized when the stage
changes afterwards. The placeholder can therefore be larger than the fitted
original and does not follow immersive mode or viewport resizes, so the preview
is mis-scaled again even though the helper-level tests still pass.

## What Changes

- Recompute the thumbnail placeholder dimensions from the current stage size
  whenever the stage layout changes while a preview is visible, including after
  the caption chrome is filled, on immersive mode changes, and on viewport
  resizes.
- Keep the placeholder square sized to the shorter edge of the original's
  fitted box and centered, so it continues to match the center crop of the
  original.
- Add a real-browser regression test that opens the viewer in Chromium, measures
  the rendered placeholder against the rendered stage for landscape and portrait
  items, and re-checks it after entering immersive mode and resizing the
  viewport.
- Wire the browser test into the existing Rust test entry point and CI, skipping
  it with a clear message when Node or a browser is unavailable.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `media-viewer`: the "Present images with pan and zoom" requirement gains
  placeholder behaviour for stage layout changes, and its thumbnail scenarios
  move from helper-level sizing to rendered-layout verification.

## Impact

- `assets/viewer.js`: placeholder sizing and stage measurement in `showItem`,
  `measureFit`, the resize handler, and `setImmersive`.
- `tests/js/`: a browser regression script and its fixture page.
- `tests/viewer.rs`: runs the browser regression and skips it when unavailable.
- Root npm tooling: Playwright dev dependency, lockfile, and a browser test fixture.
- `.github/workflows/ci.yml`: installs the browser dependency so verification
  runs in CI.
- `README.md` and `.gitignore`: development instructions and local artifacts.
