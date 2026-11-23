# Tasks

## 1. Slideshow foundation and pure helpers

- [x] 1.1 Add `SLIDESHOW_INTERVAL_MS` (5000) and an idle-chrome interval constant, plus slideshow state (`playing`, timer) and immersive state (`immersive`, idle timer) to the viewer state; verify the viewer still initializes with no console errors.
- [x] 1.2 Extract a pure next-target helper covering normal advance, stop at the last item, and restart from the first item; expose it in `__mediaCatalogViewer` and verify it passes new cases in `tests/js/viewer_check.mjs` via `node tests/js/viewer_check.mjs assets/viewer.js`.
- [x] 1.3 Assert the new helper values and `SLIDESHOW_INTERVAL_MS` in `tests/viewer.rs`; verify with `cargo test --test viewer`.

## 2. Immersive layout

- [x] 2.1 Add the immersive toggle control with an accessible name and `aria-pressed`, bind the `f` key, and keep it enabled alongside the existing controls; verify the control is reachable by Tab and reports its state.
- [x] 2.2 Add `.viewer.is-immersive` styles (full viewport, no border/radius, black background), idle chrome hiding with transitions, and restore-on-interaction styles; verify desktop and phone-sized viewports fill the window without clipping the stage.
- [x] 2.3 Schedule chrome hiding after inactivity only in immersive mode, reveal it on pointer move, tap, wheel, key press, and `focusin`, and skip hiding while a control owns focus; verify hidden controls reappear and remain keyboard-reachable.
- [x] 2.4 Make Escape leave immersive mode without closing the viewer, with the next Escape closing it; verify with the toggle, the `f` key, and Escape in both modes.
- [x] 2.5 Update the viewer CSS assertions in `tests/server.rs` to cover the immersive class and verify with `cargo test --test server`.
- [x] 2.6 Document immersive mode, the `f` shortcut, and the Escape behaviour in the viewer section of `README.md`; verify the documented steps match the manual checklist.

## 3. Automatic slideshow

- [x] 3.1 Add the play/pause control with an accessible name and `aria-pressed`, and start playback only from the control or slideshow key; verify opening the viewer does not start a timer.
- [x] 3.2 Advance image items on the 5000 ms timer using the existing navigation path so the album URL and position line update as manual navigation does; verify the timer advances and clears on each transition.
- [x] 3.3 Stop at the last item and restart from the first item when play is activated on the last item; verify both boundaries on a two-item and a single-item album.
- [x] 3.4 Pause playback on manual navigation, zoom, pan, opening the original, and `visibilitychange`; resume only through an explicit play action; verify no timer remains after each pause.
- [x] 3.5 Pause when a video item is reached, keep it paused with its poster and native controls, and make play on a video advance to the next item and resume there; verify no video autoplays and playback continues after the video.
- [x] 3.6 Clear the slideshow timer on close and leave reopenings paused; verify closing mid-playback, reopening via a tile and via `?item=`, and confirming no stale timer fires.
- [x] 3.7 Add slideshow helper assertions to `tests/js/viewer_check.mjs` and `tests/viewer.rs`; verify with `node tests/js/viewer_check.mjs assets/viewer.js` and `cargo test --test viewer`.
- [x] 3.8 Document slideshow behaviour, the 5-second interval, video pausing, and the paused-on-reopen rule in the viewer section of `README.md`; verify the documented behaviour against the scenarios in the spec delta.

## 4. Accessibility and live-region polish

- [x] 4.1 Switch the position line's `aria-live` to off while playback is running and restore manual announcements when paused; verify a screen reader does not announce every automatic advance.
- [x] 4.2 Ensure the play/pause and immersive toggles expose pressed state, that focus containment includes the new controls, and that reduced-motion preferences leave the viewer usable; verify with keyboard-only navigation and the existing reduced-motion styles.
- [x] 4.3 Confirm the caption remains the dialog's accessible name and that automatic progression does not change the caption requirements; verify the caption line and dialog labelling in a browser.
- [x] 4.4 Document the accessibility behaviour (pause control, quiet automatic progression, immersive controls) in `README.md`; verify the documented claims against a keyboard and screen-reader smoke test.

## 5. Integration verification

- [x] 5.1 Run the full test suite with `cargo test` and confirm viewer, server, and rendering tests pass without snapshot changes.
- [x] 5.2 Walk the browser verification checklist with an image album, a mixed album containing a video, a search album, and a phone-sized viewport; verify immersive mode, slideshow start/pause/stop, video pausing, Escape behaviour, focus return, and the no-JavaScript fallback.
- [x] 5.3 Run `openspec validate --change viewer-fullscreen-slideshow --strict` and confirm the change and spec delta validate cleanly.

## Workflow follow-up

- Archive the change after implementation and review, then verify the archived `media-viewer` spec.
