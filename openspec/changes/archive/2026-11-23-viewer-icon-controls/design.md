# Design

## Context

See proposal.md - Why. `assets/viewer.js` builds the dialog with DOM APIs, creates one text button per action through `controlButton()`, and updates the play and immersive labels in `updatePlayControl()` and `updateImmersiveControl()`. The script is a plain IIFE embedded through `include_str!` in `src/server/assets.rs`, and the project has no frontend build step, package bundler, or icon asset route. `assets/site.css` already provides the 2.75rem minimum control target, focus styling, disabled styling, and immersive overlay layout.

## Goals / Non-Goals

**Goals:**

- Give every viewer control a conventional icon while preserving its accessible name, keyboard behavior, focus visibility, disabled state, and touch target.
- Keep the viewer a single embedded JavaScript asset with no runtime icon request and no new server route.
- Keep the icon set small, licensed, and easy to extend when a new control is added.
- Make toggle state visible through icon swaps rather than text changes.

**Non-Goals:**

- Restyling the wider viewer layout, control grouping, or immersive overlay.
- Adding icons outside the viewer.
- Introducing an icon font, SVG sprite loader, bundler, or npm runtime dependency.
- Changing keyboard shortcuts, album navigation, zoom, slideshow, or accessibility semantics.

## Decisions

### Vendor a Lucide subset instead of adding a dependency

Use **Lucide** because it is actively maintained, uses a consistent 24x24 stroke style that suits the dark viewer chrome, and publishes simple SVG path data under the ISC license. Lucide records that some icons derive from Feather and remain under MIT; the selected `chevron-left`, `chevron-right`, `x`, `zoom-in`, `zoom-out`, `maximize`, `minimize`, and `external-link` icons are among those Feather-derived icons. Include both notices in a new `THIRD_PARTY_NOTICES.md` and a short source comment in `assets/viewer.js`.

- Why: the project has no bundler, so importing `lucide` would add build tooling for eleven paths; a sprite file would add a server route, a second request, and more cache/test surface. Inline path data keeps one asset and one request.
- Alternatives considered: Feather (same visual family but less actively maintained), Bootstrap Icons (filled style and larger set than needed), Heroicons (similar but different license posture), Material Symbols (font/asset delivery and heavier runtime).

### Keep icon geometry in a small map and render SVG nodes

Add a small `ICONS` map keyed by control name with Lucide path data and a `createIcon(name)` helper that builds an `<svg class="viewer-icon" viewBox="0 0 24 24" aria-hidden="true" focusable="false">` with path children through `document.createElementNS`. `controlButton()` appends the icon instead of text. `setIcon()` replaces the icon when a toggle changes.

- Why: `createElementNS` avoids `innerHTML`, keeps a single source of truth for shapes, and lets the browser cache the embedded script normally.
- Icon mapping:
  - close → `x`
  - previous → `chevron-left`
  - next → `chevron-right`
  - play/pause → `play` / `pause`
  - zoom out/in → `zoom-out` / `zoom-in`
  - reset zoom → `rotate-ccw`
  - immersive enter/exit → `maximize` / `minimize`
  - open original → `external-link`

### Preserve accessible names and add a visible label affordance

Keep every control's existing `aria-label` values, including the state-specific `Pause slideshow` / `Play slideshow` and `Exit immersive mode` / `Enter immersive mode` names. Add a `data-label` attribute with the same text and use CSS to show it as a tooltip on hover and focus-visible; do not also set `title`, so the browser does not show a second native tooltip.

- Why: icons alone are ambiguous, especially the reset and immersive symbols; the tooltip makes the action discoverable for sighted pointer and keyboard users without reintroducing visible text into the control row.
- Edge handling: align the first and last control tooltips inward so they are not clipped by the dialog's `overflow: hidden`.
- Alternatives considered: native `title` only (no keyboard-focus display), permanent text labels (defeats the request), or a separate legend (adds chrome and complexity).

### Style icons as part of the existing control system

Add `.viewer-icon` sizing/stroke rules and make `.viewer-button` and the original link square icon targets with centered content. Keep the existing `min-width`/`min-height`, hover, focus-visible, disabled, and reduced-motion rules; no new motion is introduced.

- Why: the current 44px targets and focus affordances already work, so the change is limited to icon geometry and spacing.
- The icon uses `stroke: currentColor` and `fill: none`, so hover and disabled colors continue to come from the control styles.

### Verify through embedded-asset and browser assertions

Extend `tests/server.rs` to assert the embedded viewer asset contains icon markup and the stylesheet contains `.viewer-icon` and tooltip rules. Extend `tests/js/viewer_browser.mjs` to assert, after opening the viewer, that every control and the original link contains an icon, has no visible text, has a non-empty accessible name, and swaps icons when play and immersive states change. Keep the existing geometry regression intact.

- Why: these are the observable behaviors in the spec and they run without a real catalog.
- Alternative considered: a Node-only unit test of path data; it would not verify that the controls actually render icon-only UI.

## Risks / Trade-offs

- [Icons are ambiguous without labels] → Keep `aria-label`, add hover/focus tooltips, and use conventional symbols for play, pause, close, and navigation.
- [Tooltips clip at the dialog edge] → Align edge tooltips inward and keep them short; verify at phone width.
- [Licensing obligations] → Add `THIRD_PARTY_NOTICES.md` with ISC and Feather-derived MIT notices plus a source comment.
- [Icon-only controls reduce touch clarity] → Preserve the 44px target and use familiar navigation/zoom symbols; accessible names still carry the action.
- [Duplicating path data in JavaScript] → Keep the map minimal and colocate a comment pointing at the Lucide icon names so future updates are traceable.
- [Accidental text remnants after refactor] → Browser assertion checks visible text is empty for icon controls.

## Migration Plan

No data or API migration. The embedded viewer asset changes in place; its existing content ETag cache behavior handles browser refresh. Rollback is reverting the asset/CSS/notice/test/docs changes.

## Open Questions

None.
