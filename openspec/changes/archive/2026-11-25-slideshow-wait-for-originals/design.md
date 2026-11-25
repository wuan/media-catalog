# Design

## Context

See `proposal.md` for motivation and `specs/media-viewer/spec.md` for the behavior contract.

The viewer is a single vanilla-JS module (`assets/viewer.js`) driving a `<dialog>`. Image opening currently:

- paints the item's thumbnail as `is-placeholder`,
- schedules `loadOriginal()` after `ORIGINAL_DELAY_MS`,
- swaps the decoded original in once `decode()` resolves.

Slideshow playback uses an independent 5-second timer that navigates with `goTo()` regardless of load state, and pauses when it reaches a video. The stage is black (`assets/site.css:420`), and manual navigation must keep its current preview behavior.

The media server serves originals with `Cache-Control: private, no-cache` and ETags (`src/server/media.rs:73`), and routes on `uri.path()`, so a query-string cache-buster still resolves to the same file.

## Goals / Non-Goals

**Goals:**

- Slideshow transitions never paint a thumbnail and never blank the stage.
- Advance only after the next image is downloaded and decoded.
- Make the usual wait invisible by prefetching the next image during the dwell.
- Skip videos in playback while leaving manual navigation untouched.
- Fail predictably after bounded retries.

**Non-Goals:**

- Changing manual-navigation preview behavior or placeholder geometry.
- Changing video playback behavior outside slideshow.
- Adding a server endpoint, cache policy, or new dependency.
- Preloading more than one slide ahead.

## Decisions

### Prepare, then commit

The timer will no longer call `goTo()`. `advanceSlideshow()` resolves the next **image** index, and commits the navigation only when that item's original is decoded:

```
timer fires -> prepare(next)
                 | ready  -> commit: replace image, set index, chrome, URL, restart dwell
                 | not ready -> stay on current image; commit when decode resolves
```

Committing index/chrome/URL at swap time keeps the caption, position indicator, and URL describing the image actually on screen. Alternative considered: advance index immediately and paint black — rejected because it shows stale chrome and contradicts the desired hold-and-swap behavior.

### Prefetch the next image target

When an image slide paints, start downloading the next image item's original. Store the `Image` element and adopt that same element on commit. This avoids both a revalidation round trip and a second decode; `decode()` on the prefetched element happens in the background so commit is usually instant.

- Prefetch only the next target, not both neighbours, and only after the current image paints, so the current slide keeps bandwidth priority.
- Only image targets are prefetched; the slideshow never opens videos.
- Skip prefetch when `navigator.connection.saveData` is set.
- When the slideshow stops, pause, or closes, cancel the timer and discard unfinished prefetch state.

Alternative considered: prefetch via `fetch` + blob URLs. Rejected as more machinery than needed; holding one `Image` element bounds memory and benefits from the HTTP cache.

### Slideshow target = next image item

Replace adjacent-index arithmetic with a helper that scans forward for `kind === "image"`.

- Start on an image: keep it.
- Start on a video: move to the next image, else the first image.
- No image ahead: stop at the last image.
- Fewer than two image items: no timer.
- Restart from the last item: first image.
- `next`/`prev` buttons still visit videos manually and pause playback as today.

### Retry ladder with per-attempt timeout

Each slideshow original load is an attempt with a hard timeout (10 s) plus `onerror`/decode rejection handling:

```
attempt 1 -> backoff 500ms -> attempt 2 -> backoff 1s -> attempt 3 -> fail
```

Retry URLs carry a cache-busting query (for example `?slideshow-retry=2`). Because responses use `no-cache`, a plain retry would revalidate and could reuse the same corrupt body; the unique query forces a fresh fetch while `uri.path()` still maps to the same media file.

On exhaustion: stop playback and open the item through the normal paused path, so its thumbnail preview, caption, error note, and original link appear. Reuse the existing `.viewer-error` element and extend its text for images rather than adding new chrome.

### Timer and state ownership

The dwell timer starts only after a paint (initial slide included) and only while playing. Manual interaction, pause, close, immersive changes, or a failed load cancels the pending commit and timer. Prefetch state lives beside `originalLoader` and is cleared by the same cancel path.

## Risks / Trade-offs

- **Two decoded originals in memory** -> Keep at most current + next; drop the prefetch on commit failure, pause, or close.
- **Prefetch competes with the current slide** -> Start it after the current image paints, not at navigation time.
- **Hung requests would stall playback** -> Per-attempt timeout ensures the retry/failure path is reachable.
- **Retry may return cached bad bytes** -> Cache-busting query on retries.
- **Large slideshow files on metered connections** -> Honor `saveData` by skipping prefetch; advance then waits for the download.
- **Test timing flakiness** -> Keep the target-selection helper pure and test it in `viewer_check.mjs`; drive decode/timing scenarios in the browser test with the existing `decode()` delay hook.
- **Chrome/URL describing the wrong slide** -> Commit index and chrome only with the image swap.
