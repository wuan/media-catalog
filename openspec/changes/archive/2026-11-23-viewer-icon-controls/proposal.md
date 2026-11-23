# Proposal

## Why

The viewer controls currently spell out actions such as `Previous`, `Next`, `Play`, `Reset`, and `Immersive`, which makes the control row wide and visually noisy over full-area media. Icon-only controls are the conventional pattern for a media viewer and free horizontal space for the media itself.

An icon library gives the viewer a coherent, familiar symbol set without drawing custom artwork. The project has no frontend build step and embeds its assets in the server binary, so using a small vendored subset of an open-source library keeps the change dependency-free: no npm package, bundler, icon font, or extra network request.

## What Changes

- Replace the visible text of every viewer control with an inline SVG icon: close, previous, next, play/pause, zoom out, zoom in, reset zoom, immersive toggle, and open original.
- Use a vendored subset of **Lucide** (ISC, with the Feather-derived icons under MIT) covering the eleven symbols the viewer needs.
- Keep each control's existing accessible name and toggle state, and add the same label as a hover tooltip so sighted pointer users can identify unfamiliar icons.
- Swap icons when control state changes: play/pause and enter/leave immersive.
- Keep controls keyboard reachable, focus-visible, disabled-state styling, and the existing minimum touch target.
- Keep the icons inline in `assets/viewer.js` so the viewer still renders with no icon-font or external icon request.
- Add third-party attribution for the vendored Lucide icons.
- Update the viewer documentation and asset/browser regression coverage for icon-only controls.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `media-viewer`: viewer controls are no longer text buttons; they must render as icon-only controls with preserved accessible names, stateful icon swaps, hover labels, and the existing keyboard, focus, disabled, and touch-target behavior.

## Impact

- `assets/viewer.js`: control construction and state updates use inline SVG icons instead of text nodes.
- `assets/site.css`: icon sizing/stroke, square icon buttons, focus and disabled styling, and the original-link icon layout.
- `THIRD_PARTY_NOTICES.md` (new): Lucide ISC and Feather-derived MIT notices.
- `tests/server.rs`: assertions for the embedded icon markup and viewer styles.
- `tests/js/viewer_browser.mjs` and/or `tests/js/viewer_check.mjs`: coverage for icon controls, accessible names, and stateful icon swaps.
- `README.md`: describe the icon controls and their labels/tooltips.
- No new server routes, templates, catalog data, deployment, or API changes.
