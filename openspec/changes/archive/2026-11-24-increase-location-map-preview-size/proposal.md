# Proposal

## Why

The hover map preview is legible but small: a 260x150 frame makes the
OpenStreetMap embed cramped next to a full-size viewer. Publishing stays the
same, so this is purely a presentation size adjustment.

## What Changes

- Enlarge the map preview frame about 1.5x: 390px wide and 225px tall instead of
  260x150, preserving today's 260:150 aspect ratio.
- Derive the frame height from its width so the popover shrinks with the window
  instead of overflowing a narrow viewer (`min(390px, calc(100vw - 2rem))`).
- Update the clamp fallback constants in `assets/viewer.js` to match.
- Update the browser regression's popover height bound and keep the narrow
  viewport and immersive placement assertions passing.
- Leave the map bbox unchanged: the larger frame shows the same geographic area
  at more detail, not more area.

Not in scope: changing the preview trigger, request policy, dismissal, placement
side, bbox span, or the OpenStreetMap embed URL.

## Capabilities

No spec-level behavior changes. The preview contract - intent-gated reveal, one
request per item, dismissal, accessible frame, and fallback caption - is
unchanged; only the rendered box size moves. The prior design left the exact
popover size tunable without a spec change, so this change sets
`skip_specs: true`.

### New Capabilities

None.

### Modified Capabilities

None.

## Impact

- `assets/site.css` - preview and frame sizing.
- `assets/viewer.js` - `PREVIEW_WIDTH` / `PREVIEW_HEIGHT` clamp fallbacks.
- `tests/js/viewer_browser.mjs` - popover height assertion after the resize.
- No generator, server, config, JSON, or dependency changes; existing output is
  byte-identical apart from the stylesheet and viewer script.
