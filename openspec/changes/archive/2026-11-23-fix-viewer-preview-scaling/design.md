# Design

## Context

The viewer's thumbnail preview is sized once in `renderImage` from `state.stageWidth`/`state.stageHeight`. `showItem` calls `measureStage()` before it fills the caption, metadata, and location chrome, so the measurement can describe a taller stage than the one that finally renders. Later layout changes are also missed: the window resize handler and `setImmersive` re-measure the stage for pan/zoom but never resize the preview. The helper-level tests in `tests/js/viewer_check.mjs` and `tests/viewer.rs` only exercise `placeholderBox`, so they cannot catch a mismatch between the helper output and the rendered stage.

See `proposal.md` for motivation and `specs/media-viewer/spec.md` for the required behaviour.

## Goals / Non-Goals

**Goals:**
- The rendered preview square matches the shorter edge of the fitted box for the stage that is actually visible.
- The preview follows caption/chrome layout changes, immersive mode, and viewport resizes.
- A real Chromium test reproduces the regression and verifies the rendered geometry automatically.

**Non-Goals:**
- Changing thumbnail generation, the center-crop source, or the original image's fitted display.
- Changing pan/zoom math or slideshow behaviour.
- Adding visual snapshot testing or pixel-diff golden images.

## Decisions

### Decision: Size the preview from the current rendered stage, not a cached measurement

Add a single preview-sizing function that reads `state.stageWidth`/`state.stageHeight`, returns early unless the current media is an `img.is-placeholder`, and applies the square dimensions from `placeholderBox`. `showItem` will populate caption and metadata chrome before measuring the stage and sizing the preview. This removes the stale measurement that causes the regression.

Alternative considered: reorder only `measureStage()` after the caption writes. Rejected because immersive mode and viewport resizes would still leave the preview stale.

### Decision: Observe the stage with `ResizeObserver`

Create a `ResizeObserver` on `.viewer-stage` when the dialog is created. Its callback re-measures the stage, refreshes fit/pan bounds, and re-applies the preview size while the preview is visible. This catches chrome growth, font-driven reflow, immersive mode, viewport resizes, and future CSS changes without enumerating every cause.

Alternative considered: call the preview sizing explicitly from the resize handler and `setImmersive` only. Rejected because it leaves other layout changes uncovered and repeats the same class of bug. The explicit calls remain as a synchronous first paint path; the observer is the safety net.

Observers are disconnected/ignored when the stage has no area, and only run while an image preview is active, so they do not affect closed dialogs or zoom transforms.

### Decision: Keep the test in a real browser and drive it from the existing Rust test entry point

Add `tests/js/viewer_browser.mjs` using Playwright with bundled Chromium. The script builds a fixture page with landscape and portrait tiles, adds `assets/site.css` and `assets/viewer.js`, clicks a tile, and compares `getBoundingClientRect()` of the rendered preview against an independently computed fitted box from the stage rect and tile dimensions. It then enters immersive mode, resizes the viewport, and checks the portrait item.

The script exits with code `77` when Playwright or a browser is unavailable. `tests/viewer.rs` treats that as a skip with a diagnostic, matching the existing Node and ffmpeg skip pattern, so local `cargo test` remains usable without browser tooling.

Alternative considered: extend the Node `vm` harness. Rejected because it cannot reproduce CSS/flex layout bugs, which is exactly the failure mode.

Alternative considered: Rust-only layout simulation. Rejected because it would duplicate browser layout rules and would not verify the real rendering path.

### Decision: Use Playwright as a project dev dependency

Add a root `package.json`/`package-lock.json` with Playwright, ignore `node_modules`, and install Chromium in CI before `cargo test`. This keeps the browser version deterministic and gives developers one documented setup command. The CI test job and coverage job both run `cargo test`, so both must install the dependency or the browser test will skip.

Alternative considered: use the runner's system Chrome through `playwright-core`. Rejected because browser availability varies across developer machines and runner images, and a missing browser would silently skip the regression.

## Risks / Trade-offs

- [ResizeObserver callback and preview sizing could feed back into layout] → The stage has `min-height: 0` and `overflow: hidden`, and the preview is sized within that fixed stage, so writing its dimensions does not change the stage box. Observer callbacks are guarded to image previews only.
- [Browser pixels are fractional and assertions are brittle] → Compare geometry with a small tolerance (for example 1.5 CSS pixels) rather than exact equality.
- [CI adds download time for Chromium] → Cache the Playwright browser directory and npm cache in CI; the test still skips locally when the browser is absent.
- [The test fixture diverges from generated pages] → Use the real `site.css` and `viewer.js`, and include caption/metadata chrome in the fixture so the layout pressure matches production.
- [Original loading changes the box during measurement] → Point fixture originals at an unavailable path so the placeholder remains visible throughout the geometry assertions, or assert only during the preview window.

## Migration Plan

No data migration. Deploy the asset change with the next catalog publish; rollback is redeploying the previous assets. The browser test is additive and skips when tooling is unavailable, so it does not block builds that lack Node or browsers.

## Open Questions

None.
