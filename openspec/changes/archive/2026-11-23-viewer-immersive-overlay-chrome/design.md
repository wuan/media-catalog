# Design

## Context

See proposal.md - Why. The viewer dialog contains a `.viewer-frame` flex column with four rows: `.viewer-top`, `.viewer-stage`, `.viewer-controls`, and `.viewer-caption`. Immersive mode already sizes the dialog to the viewport, but the chrome rows keep their layout height; fading them to `opacity: 0` leaves the stage at its framed size.

`measureFit()` fits images to the stage's bounding rect, and `setImmersive()` re-measures on entry and exit. `resolveChrome` uses an `is-chrome-hidden` class on the dialog.

## Goals / Non-Goals

**Goals:**

- Give the stage the full immersive viewport so images and videos fit the maximum available area.
- Present top bar, controls, and caption as one overlay layer that fades away and back without changing the stage's size.
- Keep the framed layout, fit/zoom/pan math, keyboard behaviour, and slideshow untouched.

**Non-Goals:**

- Changing the idle-hide timing, keyboard shortcuts, or accessibility behaviour.
- Reworking the framed (non-immersive) dialog layout.
- Adding gesture handling to the overlay.

## Decisions

### Group controls and caption in a bottom chrome block

Wrap `.viewer-controls` and `.viewer-caption` in a new `.viewer-bottom` element created in `viewer.js`. In the framed layout the wrapper is a plain block, so the visual order and spacing stay the same.

- Why: one absolutely positioned bottom block has a predictable box, so the overlay can sit at the bottom without measuring the controls' wrapped height.
- Alternatives considered: positioning the caption above controls with a computed offset (fragile when controls wrap on narrow screens), or stacking both in the same CSS grid cell (they would overlap).

### Overlay the chrome in immersive mode only

With `.viewer.is-immersive`, make `.viewer-frame` `position: relative`, take the chrome rows out of flow, and position `.viewer-top` at the top and `.viewer-bottom` at the bottom. The stage keeps `flex: 1` and therefore fills the frame. Add translucent gradient backgrounds and a z-index so overlay text stays readable over bright media; keep pointer events on the chrome but only within its own boxes.

- Why: the stage size becomes independent of chrome visibility, matching "maximum available area".
- Alternatives considered: letting the stage grow only while chrome is hidden. That shrinks the image whenever chrome appears and requires re-fitting on every fade, producing repeated layout jumps.

### Fade remains the existing class, with the bottom block included

Extend the `is-chrome-hidden` opacity/pointer-events rule to `.viewer-bottom`, and leave the hide/reveal timers alone.

- Why: no layout change occurs on fade, so no re-measure or transform reset is needed when chrome hides.
- Alternative considered: animating stage dimensions during the fade; rejected for jank and because the image would rescale while the viewer is idle.

### Re-measure only on immersive entry/exit and resize

Continue using `measureStage()` and `measureFit()` from `setImmersive()` and the existing `resize` listener. Hiding/showing chrome does not call them.

- Why: the stage rectangle is constant in immersive mode regardless of chrome state.
- Risk handled: the fit box must use the full-window stage as soon as immersive mode is entered, which the existing re-measure already covers.

## Risks / Trade-offs

- [Overlay chrome covers part of the media while visible] → Keep the chrome compact, use a gradient backing, and rely on the existing idle auto-hide; the stage remains full-sized underneath.
- [Wrapped controls plus caption cover a large share of a phone screen] → Verify phone-sized viewports and use safe-area padding so the overlay stays compact and tappable.
- [Hidden chrome remains in the accessibility tree and keyboard order] → Keep the existing rule that skips hiding while a control owns focus, and reveal chrome on `focusin`; do not add `display: none` or `visibility: hidden`.
- [Framed-mode regressions from the new wrapper] → Keep the wrapper unstyled for framed mode and verify the framed dialog in the browser checklist.

## Migration Plan

Static assets only. Deploy the updated `viewer.js` and `site.css`; the server already serves them with cache validators. Roll back by restoring the previous assets. No catalog, database, or routing migration.

## Open Questions

None.
