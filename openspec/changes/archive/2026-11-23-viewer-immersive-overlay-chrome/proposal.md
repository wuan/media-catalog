# Proposal

## Why

Immersive mode currently hides the viewer chrome by fading it out, but the chrome still occupies layout space. The stage therefore keeps the height of the framed layout, so the image never grows into the area the hidden top bar, controls, and caption leave behind. Immersive mode should give the media the whole window.

## What Changes

- In immersive mode, lay the stage out across the entire viewer so images and videos are fitted to the maximum available area.
- Turn the top bar, controls, and caption into an overlay above the stage instead of stacked layout rows.
- Keep the existing idle fade-out and reveal-on-interaction behaviour, and let it reveal the unobstructed stage rather than reclaim space.
- Keep the framed (non-immersive) layout unchanged.
- Keep the image fit, zoom, pan, and swipe calculations aligned with the full-window stage.
- Keep the chrome readable over media by giving the overlay a subtle gradient/translucent backing.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `media-viewer`: the immersive layout requirement now requires a full-window stage with overlaid chrome rather than a stage sized by stacked chrome rows.

## Impact

- `assets/viewer.js`: wrap the controls and caption so the overlay can be positioned as one bottom block, and keep immersive fit measurement in sync with the full-window stage.
- `assets/site.css`: immersive overlay positioning, stage sizing, and overlay backgrounds; framed styles stay as they are.
- `tests/server.rs`: asset assertions for the immersive overlay styles.
- `README.md`: describe the full-area immersive layout.
- No server routes, templates, catalog format, or deployment changes.
