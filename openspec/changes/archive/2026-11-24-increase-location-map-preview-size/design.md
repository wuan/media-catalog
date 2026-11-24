# Design

## Context

See `proposal.md` - Why. The preview is a fixed-position popover built in
`assets/viewer.js` and sized entirely by `assets/site.css`. Its placement code
clamps it to the dialog bounds using `PREVIEW_WIDTH` / `PREVIEW_HEIGHT` only as
fallbacks; the real size comes from `offsetWidth` / `offsetHeight` once visible.
The current frame is 260x150 and the popover computes to about 260x184.

The browser regression drives the popover through hover, focus, blocked embed,
narrow viewport, and immersive placement, and asserts both containment and a
hardcoded popover height range.

## Goals / Non-Goals

**Goals:**

- A 1.5x frame at desktop widths with the existing 260:150 aspect preserved.
- The popover remains inside the dialog at every viewport the viewer supports,
  including the 375px regression case.
- One sizing source of truth for CSS and the JS clamp fallbacks.

**Non-Goals:**

- Changing the bbox, zoom level, or geographic coverage.
- Scaling the caption type or padding with the frame; only the map area grows.
- Adding viewport-specific placement rules or a visitor-facing size control.

## Decisions

### 1. Responsive width cap, ratio-derived height

```css
.viewer-map-preview { width: min(390px, calc(100vw - 2rem)); }
.viewer-map-frame   { width: 100%; aspect-ratio: 260 / 150; height: auto; }
```

390px is 1.5x the current 260px. The `min()` cap prevents the clamp from
pinning the popover to `left + gap` and overflowing; at 375px the popover becomes
343px wide and the frame 198px tall. Deriving height from `aspect-ratio` keeps
the map undistorted at every width, which a fixed 225px height would not.

Alternatives considered:

- **Fixed 390x225 with no cap** - rejected: the narrow-viewport assertion and
  real phone-width windows overflow the dialog.
- **Keep fixed height, add a narrow media query** - rejected: two rules to keep
  in sync and an intermediate width would squash the map.
- **`padding-top` ratio box** - rejected: more markup and no benefit over
  `aspect-ratio`, which the project's CSS baseline already assumes (`dvh`, `min()`).

### 2. Bbox unchanged, coverage stays the same

The OSM embed fits the requested bbox to the frame, so a larger frame renders
the same area with more pixels. Keeping `PREVIEW_LAT_SPAN` at 0.006 leaves the
map semantics and the existing bbox assertion untouched.

Alternative considered:

- **Widen the bbox by 1.5x too** - rejected: that changes what the preview
  communicates, not just how large it is; the user chose the popover-only option.

### 3. Mirror the new size in the clamp fallbacks

Update `PREVIEW_WIDTH` to 390 and `PREVIEW_HEIGHT` to about 259 so the fallback
matches the rendered box. These values only feed placement when `offsetWidth` /
`offsetHeight` are unavailable, but keeping them aligned avoids a stale clamp on
the first frame.

### 4. Update the regression bound, keep the assertions

`tests/js/viewer_browser.mjs` asserts a blocked-embed popover height of 150-200.
The new desktop height is about 259, so the bound becomes roughly 240-280. The
containment, caption fallback, one-request, dismissal, and immersive checks stay
unchanged and must still pass at 1280px and 375px.

## Risks / Trade-offs

- **Narrow viewport overflow** → `min()` width cap plus ratio-derived height;
  covered by the existing 375px containment assertion.
- **Taller popover crowds short framed dialogs** → placement already flips below
  the link and clamps to the dialog; the framed dialog at the minimum supported
  height still leaves room above the caption.
- **`aspect-ratio` on a replaced `iframe`** → supported across evergreen browsers
  and the project's existing CSS uses comparably modern features; the regression
  measures the rendered box rather than trusting the declaration.
- **Stale height assertion masks a regression** → update the bound in the same
  change and keep the measurement-based assertion.

## Migration Plan

1. Rebuild assets so the deployed stylesheet and viewer script carry the new
   sizing; no config, JSON, or generator change is needed.
2. Roll back by restoring the previous stylesheet and viewer constants.
