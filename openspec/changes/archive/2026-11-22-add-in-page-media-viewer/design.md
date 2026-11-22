# Design

## Context

Catalog pages are server-rendered by `src/render.rs` from the published JSON
snapshot; tiles in `templates/tiles.html` are plain anchors to `/media/`. Only
`search.html` currently loads client JavaScript, and `src/server/assets.rs`
embeds assets in the binary with content-based ETags. See `proposal.md` for
motivation and the delta specs for the required behavior.

The viewer must work over three different tile layouts without new catalog data
or server routes:

```text
month page      one grid
event page      one or more grids, sectioned or month-grouped
search page     one grid rebuilt as filters change
```

## Goals / Non-Goals

**Goals:**

- One shared client-side viewer for every tile list, with the page's DOM order
  as the album.
- Deep-linkable viewer state that survives refresh, sharing, and search filters.
- Pinch/wheel/double-tap zoom and pan that do not steal album navigation.
- Inline video with native controls and a predictable keyboard rule.
- Progressive enhancement: the existing `<a href="/media/...">` fallback remains
  authoritative when scripts do not run.

**Non-Goals:**

- A per-item HTML route or server-side item detail page.
- Server-side image resizing or new preview assets beyond the current thumbnail.
- Autoplay, playlists, slideshow timing, or video trimming.
- Authentication changes for media subresources.
- Zoom/pan state persistence across items or in the URL.

## Decisions

### Client-side modal overlay, not a route

Use a modal viewer in the album page. Alternatives:

- **Per-item route** (`/2024/05/{item}`): deep-linkable and server-rendered, but
  violates the no-detail-page constraint, duplicates album context in every
  response, and needs routes for months, events, and search.
- **JSON-driven gallery app**: re-fetches data already present in the DOM and
  would have to reconstruct event section/month ordering.

The overlay keeps the server contract unchanged and makes the page itself the
album, which matches the spec.

### Album extraction from the live DOM

At open time, collect `a.tile-link` elements in document order from the current
page. This naturally covers month, event, and filtered search, and preserves the
server's ordering without duplicating it in client code.

Tiles carry minimal `data-*` attributes (relative path, kind, capture label,
thumbnail URL, dimensions when known) so the viewer does not scrape captions or
parse URLs. Server-rendered tiles and `search.js`-built tiles emit the same
attributes, keeping one selection path.

### Deep-link state via `?item=` on the album route

Store the relative path in the `item` query parameter on the existing album URL.
A fragment is rejected because event pages already use fragments for section
anchors.

- Open/navigate/close use `history.replaceState`, so albums do not accumulate
  history entries.
- Search keys (`q`, `kind`, `year`, `month`) are preserved alongside `item`.
- On load the viewer waits for its album to exist. Server-rendered pages are
  ready at DOM load; search is ready when filtering renders results. `search.js`
  dispatches a results-rendered event that the viewer listens for, with a
  DOM-scan fallback.
- A stale path leaves the album visible and opens nothing.

### Native `<dialog>` for modality and focus

Use `<dialog>` with `showModal()` for the modal boundary, focus containment, and
Escape handling. The close control, backdrop click, and Escape close the viewer;
focus returns to the originating tile, or to a sensible control for deep-linked
openings. The page behind stays inert while open.

### Pointer-gesture engine on a transform stage

The image sits on a clipped stage with `touch-action: none` and a CSS transform
(`translate` + `scale`). Pointer Events provide one code path for mouse, pen, and
touch:

- one pointer at fit: horizontal swipe threshold navigates;
- one pointer zoomed: pan, clamped to bounds;
- two pointers: pinch zoom around the midpoint;
- wheel and `+`/`-`/`0`: zoom and reset;
- double-tap/double-click: toggle fit and a zoomed scale around the gesture point.

Zoom resets on every item change. Keyboard navigation remains available while
zoomed, so pan gestures never need to double as album navigation.

### Video as a first-class item

Each video gets an inline `<video controls playsinline preload="metadata">` with
the thumbnail as poster. The file is fetched only when playback needs it.

- No autoplay.
- Navigating away pauses and rewinds, and removes the active source when the
  element is torn down.
- Arrow keys navigate unless the video element owns focus, in which case the
  native player consumes them for seeking. This matches the focus-based rule the
  spec records.
- If playback fails, the caption and direct original link remain.

### One embedded viewer asset, shared styles

Add `viewer.js` to `assets/`, served and embedded by `src/server/assets.rs` with
the same content ETag behavior as `search.js`. Extend `site.css` for the dialog,
stage, controls, captions, and reduced-motion behavior rather than adding a
second stylesheet. Month, event, and search templates load the script with
`defer`.

### Progressive fallback

The tile anchor is never rewritten into a non-link control. The viewer intercepts
plain activation only; modifier clicks and new-tab gestures keep the original
link behavior. If the script fails to initialize, the anchors continue to work.

## Risks / Trade-offs

- **[Large originals are expensive]** → Show the thumbnail immediately, fetch only
  the current image, preload adjacent images after idle, and avoid preloading
  videos. Respect data-saver hints where practical.
- **[Zoom gestures can conflict with navigation]** → At fit, swipe navigates; when
  zoomed, swipe pans. Arrow keys always remain an explicit navigation escape.
- **[Video focus can swallow arrows]** → Use the focus rule: the focused element
  owns the keys. Opening focuses the dialog frame, not the player.
- **[Bearer-token deployments cannot load subresources]** → Existing `/media/`
  links already require browser-compatible credentials; do not add header-based
  fetching in the viewer. Document this as a deployment constraint.
- **[Search results render after the viewer initializes]** → Use the results
  event plus a DOM-scan fallback before opening a deep link.
- **[Large albums mean large DOM queries]** → Query only when the viewer opens or
  the results change; no per-tile listeners.
- **[Rendered markup snapshots change]** → Expect insta snapshot updates and add
  assertions for tile metadata and script inclusion.

## Migration Plan

- Ship the viewer asset, template hooks, tile metadata, and CSS together; the
  change is additive and requires no catalog or state migration.
- Existing pages remain functional if the new asset is not deployed, because the
  anchors are unchanged.
- Rollback is reverting the template/asset changes; published JSON and state
  schemas stay compatible.
- Refresh embedded-asset validators so clients pick up the new viewer script
  without stale caching.

## Open Questions

- Exact zoom ceiling and double-tap factor are tuning values, not behavior
  changes; choose conservative defaults and adjust after use.
- Whether to preload only the next item or both neighbours can be tuned for
  data usage without altering the contract.
