# Tasks

## 1. Resize the preview box

- [x] 1.1 In `assets/site.css`, set `.viewer-map-preview` to `width: min(390px, calc(100vw - 2rem))` and `.viewer-map-frame` to `width: 100%; aspect-ratio: 260 / 150; height: auto`. Verify in a browser at 1280px that the frame measures 390x225 and no caption or padding wraps.
- [x] 1.2 Update `PREVIEW_WIDTH` to 390 and `PREVIEW_HEIGHT` to 259 in `assets/viewer.js` so the placement fallbacks match the rendered box. Verify with `grep -n "PREVIEW_WIDTH\|PREVIEW_HEIGHT" assets/viewer.js` that only these constants changed.

## 2. Keep the regression honest

- [x] 2.1 Update the blocked-embed popover height bound in `tests/js/viewer_browser.mjs` from 150-200 to 240-280. Verify the assertion message still describes the measured height.
- [x] 2.2 Run `npm run test:viewer`. Verify it passes, including hover/focus reveal, one request per item, 375px containment, blocked-embed caption fallback, and immersive placement.

## Workflow follow-up

- Archive the change after the viewer regression passes and the sizing is confirmed at desktop and narrow widths.
