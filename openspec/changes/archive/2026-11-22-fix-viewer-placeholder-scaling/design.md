# Design

## Context

The viewer shows a 512x512 square center-crop thumbnail as a placeholder while the full-resolution original loads. The placeholder is sized using `fittedBox(item.width, item.height, stageWidth, stageHeight)`, which computes a contain-fit (Math.min) box matching the original's aspect ratio. Because the thumbnail is square and the fitted box is not, `object-fit: cover` on `.viewer-image.is-placeholder` then crops the square to the box shape. The result is a placeholder that appears scaled to the shorter edge of the fitted box rather than filling the longer edge. See proposal.md for motivation.

## Goals / Non-Goals

**Goals:**
- The placeholder square thumbnail fills the longer edge of the original's fitted box, so the preview visually matches the area the original will occupy.
- No layout shift occurs when the original replaces the placeholder.
- Existing pan/zoom behavior using `item.width`/`item.height` is preserved.

**Non-Goals:**
- Changing how the original image is fitted or displayed.
- Changing thumbnail generation (still 512x512 center crop).
- Changing video poster behavior.

## Decisions

### Decision: Compute placeholder size from the longer edge of the fitted box

The placeholder dimensions will be derived from the fitted box of the original, but instead of setting both CSS width and height to the contain-fit box (which crops the square thumbnail), we set only the dimension along the longer edge of the fitted box and let the square thumbnail's other dimension follow proportionally. Concretely, for a fitted box of `bw x bh`:

- If `bw >= bh` (landscape or square fitted box): placeholder width = `bw`, height = `bw` (square, filling the longer edge = width).
- If `bh > bw` (portrait fitted box): placeholder height = `bh`, width = `bh` (square, filling the longer edge = height).

The stage has `overflow: hidden`, so the overflow along the shorter edge is clipped, and the placeholder visually fills the fitted box along the longer edge.

**Alternative considered:** Keep setting both CSS dimensions to the contain-fit box and rely on `object-fit: cover`. This is the current behavior and produces the reported mis-scaling. Rejected.

**Alternative considered:** Use `object-fit: contain` on the placeholder. This would letterbox the square thumbnail inside the fitted box, leaving visible gaps. Rejected.

### Decision: Replace `fittedBox` with a `placeholderBox` helper

Introduce a `placeholderBox(width, height, stageWidth, stageHeight)` helper that returns the square placeholder dimensions based on the longer edge of the fitted box. Keep `fittedBox` for any other callers (currently only used for the placeholder and in tests). Export the new helper on `__mediaCatalogViewer` for testing.

### Decision: CSS `object-fit: cover` stays on `.is-placeholder`

With the placeholder now sized as a square matching the longer edge, `object-fit: cover` will crop the square to the element's CSS box. Since the CSS box is now square (matching the thumbnail's aspect ratio), no cropping occurs and the thumbnail renders 1:1. The `max-width: none; max-height: none` rules remain so the explicit dimensions are respected.

## Risks / Trade-offs

- [Risk] If `stageWidth`/`stageHeight` are zero when `renderImage` runs, the placeholder falls back to default sizing. → Mitigation: `measureStage()` is called in `showItem()` before `renderImage()`, so stage dimensions are available. The helper returns null on zero inputs, same as `fittedBox`.
- [Risk] Very wide panoramas may produce a placeholder taller than the stage, clipped by `overflow: hidden`. → This is the intended visual: the placeholder fills the longer edge and the shorter overflow is hidden, matching what the original will show.

## Migration Plan

No data migration. The change is a frontend-only fix in `assets/viewer.js` and tests. Deploy with the next catalog publish.

## Open Questions

None.
