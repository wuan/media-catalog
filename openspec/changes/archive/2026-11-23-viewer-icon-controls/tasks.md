# Tasks

## 1. Vendor the icon subset and attribution

- [x] 1.1 Add `THIRD_PARTY_NOTICES.md` with the Lucide ISC notice, the Feather-derived MIT notice for `chevron-left`, `chevron-right`, `x`, `zoom-in`, `zoom-out`, `maximize`, `minimize`, and `external-link`, and links to lucide.dev; verify the file names both licenses and all selected icon names.
- [x] 1.2 In `assets/viewer.js`, add a small `ICONS` map containing the Lucide path data for `x`, `chevron-left`, `chevron-right`, `play`, `pause`, `zoom-out`, `zoom-in`, `rotate-ccw`, `maximize`, `minimize`, and `external-link`; verify the map has exactly those keys and no icon path is empty.
- [x] 1.3 Add a `createIcon(name)` helper that builds a 24x24 SVG with `document.createElementNS`, `aria-hidden="true"`, `focusable="false"`, and the `viewer-icon` class; verify a rendered control contains exactly one SVG child and the SVG is hidden from assistive technology.

## 2. Render icon-only controls

- [x] 2.1 Refactor `controlButton()` to accept an icon name, append the SVG instead of `textContent`, and update the close, previous, next, play, zoom-out, zoom-in, reset, and immersive controls to pass their icons; verify every `.viewer-button` has an SVG and no visible text.
- [x] 2.2 Replace the `Open original` text on `originalLink` with the `external-link` icon while keeping its `aria-label`; verify the link contains the icon, no visible text, and a non-empty accessible name.
- [x] 2.3 Set `data-label` on every icon control from the same text used by `aria-label`, without adding `title`, so only the CSS tooltip appears; verify each control's `data-label` matches its accessible name and the tooltip uses that attribute.
- [x] 2.4 Update `updatePlayControl()` and `updateImmersiveControl()` to swap the play/pause and maximize/minimize icons alongside the existing labels and `aria-pressed` values; verify pressing `s` swaps play to pause and back, and pressing `f` swaps maximize to minimize and back.
- [x] 2.5 Extend `tests/js/viewer_browser.mjs` to assert icon-only rendering, accessible names, label attributes, stateful icon swaps, and preserved 44px control size; run `node tests/js/viewer_browser.mjs` and confirm the existing preview geometry checks still pass.

## 3. Style icons and tooltips

- [x] 3.1 In `assets/site.css`, add `.viewer-icon` sizing, `fill: none`, `stroke: currentColor`, stroke width, and line caps; make `.viewer-button` and `.viewer-original` square centered icon targets without changing the existing 2.75rem minimum size.
- [x] 3.2 Add a hover/focus-visible tooltip for controls using `::after { content: attr(data-label) }`, with inward alignment for the first and last controls so the dialog's `overflow: hidden` does not clip labels; verify the tooltip appears on hover and keyboard focus and stays inside the viewer at phone width.
- [x] 3.3 Extend `tests/server.rs` to assert the embedded viewer asset contains `viewer-icon` markup and the stylesheet contains `.viewer-icon` and the tooltip rule; run `cargo test --test server` and confirm the asset checks pass.
- [x] 3.4 Update the viewer section of `README.md` to describe icon-only controls, tooltips, accessible names, and stateful play/pause and immersive icons; verify the described controls match the implemented labels.

## 4. Integration verification

- [x] 4.1 Run `cargo test` and confirm the viewer helper tests, server asset tests, rendering tests, and browser regression all pass.
- [x] 4.2 Walk the viewer in a browser with an image album, a video album, a search album, keyboard-only navigation, immersive mode, and a phone-sized viewport; verify each icon is recognizable, tooltips/labels appear, state icons swap, disabled controls remain readable, focus stays visible, and no control lost its touch target.
- [x] 4.3 Run `openspec validate viewer-icon-controls --strict` and confirm the change and delta spec validate cleanly.

## Workflow follow-up

- Archive the change after implementation and review, then verify the archived `media-viewer` spec.
