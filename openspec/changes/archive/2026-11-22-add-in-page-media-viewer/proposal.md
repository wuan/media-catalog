# Proposal

## Why

Catalog tiles currently link straight to raw originals, so browsing an album means
opening and closing files one at a time. Visitors want to inspect photos and play
videos in place, then move through the current album with the keyboard or a swipe
without losing their position.

## What Changes

- Add an in-page media viewer opened from any catalog tile, without adding a
  per-item HTML page or route.
- Treat the current page's tiles, in document order, as the album: a month page,
  a whole event page (crossing authored sections), or the current filtered search
  results.
- Support previous/next navigation with Left/Right, on-screen buttons, and
  horizontal swipes at fit. Navigation stops at the first and last item.
- Show images at the original resolution with fit-to-screen presentation, thumbnail
  placeholder while loading, and pinch, wheel, double-tap, keyboard, and drag
  pan-zoom gestures. Zoom resets for each newly opened item.
- Play videos inline with native controls, thumbnail poster, metadata-only
  preload, no autoplay, and pause on navigation. Arrow keys navigate unless the
  video itself owns focus, in which case they seek.
- Make viewer state deep-linkable through `?item=<relative path>` on the album
  route while preserving search filter parameters; arrows and close update the
  URL without adding history entries.
- Keep the tile link as the no-JavaScript fallback so every page remains
  browsable and opens originals directly when scripts are unavailable.
- Add an embedded viewer asset and tile metadata needed by the client, and keep
  month pages lightweight by loading only the active item and adjacent image
  previews.

## Capabilities

### New Capabilities

- `media-viewer`: the context-aware in-page viewer: album boundaries, keyboard
  and gesture navigation, image pan-zoom, inline video, deep links, focus and
  fallback behavior.

### Modified Capabilities

- `static-catalog`: item interaction now opens the in-page viewer instead of
  navigating directly to the original; the existing no-JavaScript fallback and
  lightweight month-page constraints remain.
- `media-server`: embedded assets include the viewer script alongside the search
  script, served with the same content-validator behavior.
- `event-browsing`: event item presentation exposes the viewer while retaining
  lazy tiles, escaped metadata, and direct original links for non-JavaScript
  clients.

## Impact

- `templates/tiles.html`, `templates/month.html`, `templates/event.html`, and
  `templates/search.html` gain viewer hooks and tile metadata.
- `assets/` gains viewer JavaScript and supporting styles; `src/server/assets.rs`
  serves it.
- `assets/search.js` exposes rendered results so the viewer can attach to
  filtered lists and honor search deep links.
- Rendering tests and insta snapshots change because tile markup and script tags
  change.
- No catalog JSON, generator, state, or storage-root schemas change.
