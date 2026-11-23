# Proposal

## Why

The in-page viewer currently caps its size on desktop and only offers manual
navigation, so browsing a large album means clicking or swiping through every
tile. Users want a distraction-free, hands-off way to review a month, event, or
search result set without leaving the page.

## What Changes

- Add an **immersive mode** that expands the viewer to the full window and hides
  its chrome after a short idle period. Controls reappear on pointer, touch, or
  keyboard focus. Escape exits immersive mode before closing the viewer.
- Add an **automatic slideshow** that advances through the album every 5 seconds
  with a visible play/pause control. It never auto-starts, stops at the last
  item, and pauses on manual navigation, zoom/pan, tab changes, or close.
- When the slideshow reaches a video, it pauses and leaves the video waiting for
  the user to play it; pressing Play on a video advances to the next item and
  resumes the slideshow.
- Keep the slideshow and immersive mode independent: either can be used alone.
- Suppress the position line's live announcements while auto-advancing, and keep
  the existing caption, URL, focus-return, and no-JavaScript behaviour unchanged.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `media-viewer`: adds requirements for the immersive layout and the automatic
  slideshow, and extends the accessibility requirement to cover pausing playback
  and reduced-motion-safe announcements.

## Impact

- `assets/viewer.js`: viewer state, controls, timer, keyboard handling, and
  teardown.
- `assets/site.css`: immersive layout, idle chrome hiding, and mobile rules.
- `tests/js/viewer_check.mjs`, `tests/viewer.rs`, `tests/server.rs`: helper and
  asset assertions.
- `README.md`: document the new viewer modes.
- No server routes, templates, rendering, catalog format, or deployment changes:
  the assets are already embedded and loaded on every album page.
