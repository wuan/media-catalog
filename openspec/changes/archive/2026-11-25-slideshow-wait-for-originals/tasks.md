# Tasks

## 1. Slideshow target selection

- [x] 1.1 Add an exported pure helper that returns the next image item in album order, skipping `kind === "video"`, covering restart-from-last, start-on-video, and no-remaining-image cases; verify the new `viewer_check.mjs` assertions pass through `cargo test --test viewer`
- [x] 1.2 Replace the slideshow timer's adjacent-index calculation with the new helper in `startSlideshow`/`advanceSlideshow`, remove the pause-on-video branches, and verify the existing slideshow/interaction browser assertions still pass with `npm run test:viewer`

## 2. Prepare-then-commit transitions and prefetch

- [x] 2.1 In slideshow playback, request each new slide's original immediately without painting a thumbnail or placeholder, and verify the browser test sees no `.viewer-image.is-placeholder` for a playback slide
- [x] 2.2 Commit the slide only after its original decodes: update the image, index, caption, position, and URL together, and start the 5-second dwell after the paint; verify a decode-delayed browser test keeps the previous image visible and shows the new item only after release
- [x] 2.3 Prefetch the next image item's original after the current image paints, adopt the same prefetched element on commit, and skip prefetching when `navigator.connection.saveData` is set; verify with a browser test that the next slide commits without a new thumbnail or blank frame
- [x] 2.4 Cancel the pending commit, dwell timer, and prefetch when playback pauses, the visitor navigates/zooms/opens the original, the document hides, or the viewer closes; verify no late slide or stale original appears in the browser test

## 3. Bounded retries and failure state

- [x] 3.1 Add the retry ladder for slideshow originals: 3 attempts, 10-second timeout per attempt, 500 ms then 1 s backoff, and a cache-busting query on retries; verify a failing route records the expected requests in a browser test
- [x] 3.2 On exhausted retries, stop playback and open the failed item through the normal paused path with its thumbnail preview, caption, error note, and original link; verify the browser test asserts the play control resets, no timer remains, and the error note is visible
- [x] 3.3 Add the retry/failure browser scenario to `tests/js/viewer_browser.mjs` and verify it passes with `npm run test:viewer`

## 4. Documentation and integration

- [x] 4.1 Update the README slideshow description and manual verification steps to cover decoded-original waits, skipped videos, and the failure state; verify the described behavior matches the implemented controls
- [x] 4.2 Run `cargo test`, `cargo fmt --check`, `cargo clippy -- -D warnings`, and `npm run test:viewer`, and verify all pass

## Workflow follow-up

- Review the implemented change against `specs/media-viewer/spec.md` before archiving.
- Archive the change after review is complete.
