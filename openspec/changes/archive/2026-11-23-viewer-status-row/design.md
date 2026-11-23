# Design

## Context

See `proposal.md` for motivation and `specs/media-viewer/spec.md` for the required behavior.

The viewer builds its chrome in `assets/viewer.js`. The caption is currently a `.viewer-caption` container holding three sibling paragraphs: `.viewer-captured` (dialog accessible name), `.viewer-meta` (camera settings), and `.viewer-location` (OpenStreetMap link). `assets/site.css` gives each paragraph its own top margin, producing three stacked lines. The caption and settings text are already produced by `captionText()` and `metadataText()`, and coordinates by `formatCoordinates()`; those formatters do not need to change.

The browser regression fixture (`tests/js/fixtures/viewer-browser.html`) provides an item with a caption, complete camera settings, and coordinates, so it can verify the row without new fixture data.

## Goals / Non-Goals

**Goals:**
- Render caption, coordinates, and camera settings as one status row on normal viewports.
- Centre the coordinate segment relative to the caption row and keep camera settings flush right.
- Preserve existing optionality rules and accessibility relationships.
- Keep the row readable on narrow viewports without horizontal scrolling.

**Non-Goals:**
- Changing caption, settings, or coordinate formatting.
- Changing tile captions, catalog data, or the OpenStreetMap URL.
- Adding a map embed or any network request before the location link is activated.

## Decisions

### Use a three-column CSS grid for the status row

Add a `.viewer-status` grid container inside `.viewer-caption` and use:

```css
.viewer-status {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto minmax(0, 1fr);
  align-items: baseline;
  gap: 0.35rem 1rem;
}
```

- `.viewer-captured` starts in column 1 (`justify-self: start`).
- `.viewer-location` sits in column 2 (`justify-self: center`, `text-align: center`).
- `.viewer-meta` ends in column 3 (`justify-self: end`, `text-align: right`).

The two equal `1fr` side columns make the auto-width centre column align with the row's centre regardless of how wide the caption or settings text becomes. `minmax(0, 1fr)` also lets the side text wrap instead of overflowing. All segment top margins become zero; the row's gap owns spacing.

**Alternative considered:** a flex row with `flex: 1` on the outer segments. It is simpler, but a flex row only keeps the centre element exactly centred while the outer items have equal widths; long settings text would push the centre off-centre. The grid keeps centring deterministic.

### Keep the existing segment nodes and accessibility hooks

DOM order becomes `captured`, `location`, `meta`, matching the visual left-to-right order. The existing ids stay:

- `aria-labelledby` continues to point at `viewer-captured`, so the capture caption remains the dialog's accessible name.
- `aria-describedby` continues to point at `viewer-caption`, so the description includes the settings and location text.

No viewer state or formatter changes are needed; `showItem()` continues to fill the same three nodes and toggles `hidden` on the optional ones.

### Hide missing segments without leaving gaps

The `hidden` attribute removes a segment from layout while the three-column grid remains in place. Therefore:

- Missing coordinates leave the caption left and settings right.
- Missing settings leave the caption left and coordinates centred.
- Both missing leave only the caption at the left.

The row does not render placeholder separators or empty links.

### Stack the row on narrow viewports

Below a narrow breakpoint (640 px, matching the existing phone-sized viewer breakpoint), switch `.viewer-status` to one column:

```css
@media (max-width: 640px) {
  .viewer-status {
    grid-template-columns: minmax(0, 1fr);
    justify-items: start;
  }

  .viewer-location,
  .viewer-meta {
    justify-self: start;
    text-align: left;
  }
}
```

This keeps the row readable and avoids squeezed columns while preserving the reading order of caption, coordinates, and settings. Side-by-side centring is the normal-viewport layout required by the spec; stacking is the narrow-viewport fallback.

## Risks / Trade-offs

- **Coordinates drift off-centre if the grid changes** → Keep the two side columns equal (`1fr`) and verify centre alignment in the browser regression test with a small tolerance.
- **Long camera settings wrap oddly on medium widths** → Allow side columns to shrink to zero and wrap; below 640 px, switch to a single-column stack.
- **Caption accessible name accidentally includes new segments** → Keep `aria-labelledby` on `viewer-captured` only, and re-check the accessible-name assertion in the regression test.
- **Placeholder sizing depends on caption height** → The viewer measures the stage after filling the chrome, so a shorter status row changes stage height but the existing measurement flow still sizes the placeholder from the rendered stage. No additional resize path is needed.

## Migration Plan

No data, API, or deployment migration. The change ships with the next build of the embedded viewer asset and stylesheet. Rollback is limited to reverting the viewer and stylesheet changes; no catalog rebuild is required.
