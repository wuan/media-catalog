# Design

## Context

The viewer (`assets/viewer.js`) opens an image item by showing the 512x512
center-crop thumbnail (`item.thumbUrl`), then swaps `img.src` to the original
(`item.mediaUrl`) once it loads. `measureFit()` derives the displayed box from
`media.naturalWidth/naturalHeight`, which is 512x512 while the thumbnail is
shown, so the box changes when the original arrives. Each tile already carries
the original's upright dimensions as `data-width`/`data-height`, parsed into
`item.width`/`item.height` by `tileFromElement`. These dimensions are
EXIF-orientation-corrected in `src/probe.rs` (`upright_dimensions`), matching
the orientation applied to the thumbnail, so they describe the same aspect
ratio the original will have in the viewer.

## Goals / Non-Goals

**Goals:**
- Thumbnail placeholder occupies the exact fitted box of the original image, so
  the src swap causes no layout shift.
- Pan/zoom clamping uses the original's fitted size from the moment the item
  opens, not the thumbnail's square.

**Non-Goals:**
- Changing thumbnail generation (the 512x512 center crop stays as specified by
  `thumbnail-generation`).
- Video items: the video poster display is unchanged.
- Stacked image layers: the existing single-`img` src swap is kept.

## Decisions

**Decision: size the placeholder from `item.width`/`item.height`, cover-scaled.**
While the thumbnail is displayed, compute the fitted box from the real
dimensions (`fit = min(stageW/w, stageH/h)`), set the `<img>`'s explicit CSS
width/height to that box, and apply `object-fit: cover` so the inner 512x512
crop fills the box without distortion. When the original loads, clear the
explicit sizing (or keep the same box — the original has the same aspect
ratio) and swap `src`; the box is identical, so the original appears exactly on
top of the placeholder.
- Alternative considered: generate aspect-ratio-preserving thumbnails
  (contain instead of cover). Rejected: contradicts the `thumbnail-generation`
  spec and changes grid tile rendering.
- Alternative considered: stack two `<img>` elements (thumbnail below,
  original absolutely positioned on top). Rejected: extra DOM and lifecycle
  complexity for no benefit over a src swap into a matched box.

**Decision: `measureFit` prefers the item's real dimensions while the thumbnail is shown.**
`measureFit()` will use `item.width`/`item.height` (when present) instead of
the thumbnail's natural 512x512, so `baseWidth`/`baseHeight` — the pan clamp
bounds — describe the original's fitted size before and after the swap.
- Alternative considered: keep natural-dimension measurement and only fix the
  CSS box. Rejected: zooming while the placeholder is visible would clamp
  against the square box and jump when the original loads.

**Decision: fall back to current behavior when dimensions are missing.**
If `item.width`/`item.height` are absent (videos, unprobed items), the viewer
fits the thumbnail's own dimensions exactly as today.

## Risks / Trade-offs

- [Placeholder looks zoomed-in relative to the final image] → Inherent to
  showing only the center 512x512 crop; acceptable because it is transient and
  the user explicitly accepted lower resolution with cropped edges.
- [Stale or wrong width/height data in hand-built catalogs] → The box could
  mismatch the original's true aspect ratio. Mitigation: data comes from the
  probe path which is orientation-corrected; fallback only when absent.
- [Stage resize while the thumbnail is shown] → The existing window resize
  handler re-runs `measureFit`, which now uses the real dimensions, so the box
  stays consistent.

## Migration Plan

No data migration. The change is a static asset update (`viewer.js`,
`site.css`); deploy by rebuilding/republishing the catalog. Rollback is
redeploying the previous assets.

## Open Questions

None.
