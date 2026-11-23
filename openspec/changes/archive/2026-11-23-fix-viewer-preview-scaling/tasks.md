# Tasks

## 1. Add the browser regression harness

- [x] 1.1 Add a root `package.json` with Playwright as a dev dependency, commit its lockfile, and ignore `node_modules`; verify `npm install` and `npx playwright install chromium` succeed from a clean checkout.
- [x] 1.2 Add `tests/js/fixtures/viewer-browser.html` with landscape and portrait tiles, known dimensions, a 512x512 thumbnail, caption/metadata/location chrome, and an unavailable original URL; verify the fixture loads the real `assets/site.css` and `assets/viewer.js` when opened by the test.
- [x] 1.3 Add `tests/js/viewer_browser.mjs` that launches Chromium, opens the fixture, clicks the landscape tile, independently computes the expected shorter-edge square from the rendered stage rect, and asserts the rendered preview bounds, centering, and `object-fit`; verify it fails against the current `assets/viewer.js` with a measurable size mismatch.
- [x] 1.4 Extend the same script to toggle immersive mode, resize the viewport, and move to the portrait tile, re-asserting the preview bounds after each change; verify the script exits with code `77` when Playwright or the browser is unavailable.
- [x] 1.5 Add a Rust test in `tests/viewer.rs` that invokes the browser script, skips on exit code `77`, and fails with captured stderr otherwise; verify `cargo test --test viewer` reports the browser regression and skips cleanly without Playwright installed.

## 2. Fix the preview sizing

- [x] 2.1 Add a preview-sizing helper in `assets/viewer.js` that reads the current stage dimensions, applies the square shorter-edge size only while `.is-placeholder` is shown, and clears the explicit size when the original arrives; verify the Node helper output still matches `tests/js/viewer_check.mjs` and `tests/viewer.rs`.
- [x] 2.2 In `showItem`, populate the caption, metadata, and location chrome before measuring the stage and sizing the preview; verify the browser regression's initial landscape assertion passes.
- [x] 2.3 Add a guarded `ResizeObserver` on the stage that re-measures the stage, refreshes fit bounds, and re-applies the preview size while a preview is visible; verify the immersive-mode and viewport-resize assertions pass.
- [x] 2.4 Keep the existing resize handler and `setImmersive` paths synchronized with the same preview update so the first frame after a change is correct; verify the browser regression passes without frame waits beyond one animation frame.

## 3. Wire CI and document the test

- [x] 3.1 Update `.github/workflows/ci.yml` to install npm dependencies and Chromium in every job that runs `cargo test`, and cache the npm and Playwright browser downloads; verify the workflow is valid and the browser regression is not skipped in CI logs.
- [x] 3.2 Document the browser test setup and commands in `README.md`, next to the existing build and test commands; verify the documented commands run as written.
- [x] 3.3 Keep `node --check` coverage of the new script in the verify job; verify `for asset in assets/*.js tests/js/*.mjs; do node --check "$asset"; done` passes.

## 4. Integration verification

- [x] 4.1 Run `cargo test --locked --all-targets` with Playwright and Chromium installed; verify the viewer helper and browser regression tests pass together.
- [x] 4.2 Run `cargo fmt --check`, `cargo clippy --locked --all-targets -- -D warnings`, and `openspec validate --all --no-interactive`; verify all checks pass.

## Workflow follow-up

- Review the rendered preview in a running catalog at a desktop and phone-sized viewport.
- Archive the change after review requirements are satisfied.
