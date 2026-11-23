# Tasks

## 1. Chrome structure for the overlay

- [x] 1.1 In `assets/viewer.js`, create a `.viewer-bottom` wrapper around `.viewer-controls` and `.viewer-caption` and append it after the stage; verify the viewer initializes without console errors and the framed layout keeps controls above the caption.
- [x] 1.2 Add an unstyled `.viewer-bottom` block rule to `assets/site.css` so the framed dialog keeps its current spacing and order; verify the framed layout has no visual change in the browser.

## 2. Full-area immersive stage with overlaid chrome

- [x] 2.1 In `assets/site.css`, make `.viewer.is-immersive .viewer-frame` the positioning context and keep `.viewer-stage` filling the whole frame while the chrome is taken out of flow; verify the stage's bounding rect matches the immersive dialog in devtools.
- [x] 2.2 Position `.viewer-top` at the top and `.viewer-bottom` at the bottom as an overlay with translucent gradient backing, z-index, and safe-area padding; verify the stage rect is unchanged with chrome visible and that overlay text stays readable over a bright image.
- [x] 2.3 Extend the `.viewer.is-chrome-hidden` opacity and pointer-events rule to `.viewer-bottom`; verify idle chrome fades out, interaction reveals it, and the stage rect never changes during the fade or reveal.
- [x] 2.4 Add CSS assertions for `.viewer-bottom` and the immersive overlay selectors to `tests/server.rs`; verify with `cargo test --test server`.
- [x] 2.5 Document the full-area immersive stage and overlaid fading chrome in the viewer section of `README.md`; verify the documented behaviour against the spec delta scenarios.

## 3. Integration verification

- [x] 3.1 Run the full test suite with `cargo test` and confirm viewer, server, and rendering tests pass.
- [x] 3.2 Walk a browser checklist with an image album, a mixed album containing a video, a search album, and a phone-sized viewport; verify the media fits the full stage on immersive entry, the stage size stays unchanged while chrome fades, zoom/pan and swipe behaviour still work, the framed layout is unchanged, slideshow state survives leaving immersive mode, and the no-JavaScript fallback still opens originals.
- [x] 3.3 Run `openspec validate viewer-immersive-overlay-chrome --strict` and confirm the change and spec delta validate cleanly.

## Workflow follow-up

- Archive the change after implementation and review, then verify the archived `media-viewer` spec.
