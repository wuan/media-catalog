# Design

## Context

See `proposal.md` - Why. The catalog is static: the generator writes HTML and
sharded JSON, the server serves them and renders only the search shell, which it
does without generator configuration. The viewer is dependency-free vanilla JS
that reads tile data attributes, and the current coordinate link opens
OpenStreetMap in a new tab with no request beforehand.

## Goals / Non-Goals

**Goals:**

- One deployment-level switch that gates the preview and nothing else.
- No map request until hover or keyboard focus intent, and one request per item.
- Preview works on month, event, and search albums without an extra fetch on
  month or event pages and without teaching the server the generator config.
- Every existing deployment emits identical output with the flag absent.

**Non-Goals:**

- Editing or annotating the map (markers, pins, notes).
- Previews on tiles or on the year and month index pages.
- Build-time map image generation, tile caching, or offline previews.
- A visitor-facing toggle; the switch is a publisher decision.

## Decisions

### 1. A separate opt-in flag with a hard dependency on location publication

`location_preview` resolves flag-first, TOML second, default off, and validation
fails when it is enabled while `include_location` is disabled. Preview gates only
the popover; coordinates and the link are unaffected.

Alternatives considered:

- **Auto-enable with `include_location`** - rejected: publishing coordinates and
  leaking a visitor's address to a tile host are different privacy decisions.
- **Runtime toggle in the viewer** - rejected: a visitor toggle cannot protect
  the IP already sent on page load, and it adds persisted UI state for no gain.
- **Silently ignore preview without location** - rejected: a typo or stale config
  would quietly produce no preview; this project already rejects invalid config.

### 2. Two emission channels, one resolved value

`src/json.rs` writes `location_preview: true` into `catalog/index.json` when
enabled and omits it otherwise. `src/render.rs` passes the value to the month and
event templates, which put `data-location-preview="true"` on `<body>`.
`assets/search.js` copies the index value onto the same body attribute once the
index arrives. `assets/viewer.js` reads the attribute lazily at hover/focus time,
so the search page's async ordering cannot race the first preview.

Alternatives considered:

- **Index only** - rejected: month and event pages deliberately load no JSON, so
  they would need a new fetch just for a boolean.
- **Server reads the index at boot to render the search shell** - rejected: the
  server is intentionally config-blind and shell rendering happens before
  catalog availability is guaranteed.
- **Per-tile attribute** - rejected: it repeats a deployment setting on every
  item and mixes config with item metadata.

Omitting the field when off keeps the JSON privacy test's forbidden-substring
check meaningful (`location_preview` contains `location`).

### 3. Official OpenStreetMap embed in a popover

The preview is an `iframe` pointing at
`https://www.openstreetmap.org/export/embed.html` with a bbox around the point
and `marker=lat,lon`. OSM's embed owns attribution, marker rendering, and tile
math, so no map library or vendored tile code is added.

Alternatives considered:

- **Single tile image plus hand-drawn marker** - less weight, but we own tile
  math, marker placement, and attribution, and the tile policy is written for
  interactive maps rather than decorative previews.
- **Leaflet from a CDN** - rejected: new runtime dependency, larger download, and
  a second third-party origin.
- **Build-time static map** - rejected: needs a licensed static-map provider or
  systematic tile downloads, and breaks the build's offline assumptions.

Frame attributes: `loading="lazy"`, `referrerpolicy="no-referrer"`,
`title` naming the coordinates, and `tabindex="-1"` so the embedded document
never enters the modal's tab cycle.

### 4. Intent delay and one frame per item

Pointer hover starts a short (~200 ms) timer that is cancelled on leave; keyboard
focus reveals immediately. The frame is created once per item and reused, so
re-hovering never requests again. Item change discards the frame; closing the
viewer discards it with the chrome.

Alternatives considered:

- **Load on `pointerenter` immediately** - rejected: sweeping the pointer across
  the status row would fire requests.
- **Load only on click** - rejected: that is today's behavior and does not
  satisfy the hover preview.

### 5. Placement, dismissal, and failure

The popover is anchored above the location link, horizontally clamped to the
dialog, and flipped below when there is no room. It hides on pointer leave, blur,
Escape, item change, and viewer close. A small caption line inside the popover
keeps the coordinates visible if the frame never paints, so an unreachable map
host costs no layout jump and no dead box. Coarse pointers get no preview; the
link behaves as today.

Alternatives considered:

- **Fixed panel in the caption** - rejected: consumes permanent space and competes
  with the single status row.
- **Double-tap to preview on touch** - rejected: it conflicts with the link's
  primary navigation action.

### 6. Bbox derived from the frame aspect

Latitude span is fixed (~0.006 degrees); longitude span is scaled by
`1 / cos(latitude)` so the preview keeps roughly square degrees and matches the
frame's aspect. Values are computed from the already-published coordinates, so
no new metadata is stored.

## Risks / Trade-offs

- **Visitor IP reaches OpenStreetMap on intent** → preview is opt-in, default
  off, requests carry no referrer, and documentation states it plainly.
- **OSM embed outage or blocked host** → popover shows coordinates as fallback
  text; link remains the primary path.
- **Embed may set cookies in its own frame** → rejected `sandbox` because it
  breaks the embed; documented as a consequence of enabling the preview.
- **Immersive chrome auto-hides and clips the popover** → hover keeps chrome
  revealed; placement re-clamps on resize and immersive toggle.
- **Focus escaping into the frame** → `tabindex="-1"` plus assertions in the
  browser test.
- **Slideshow advances while a preview is open** → preview dismisses on item
  change.
- **Config error surprises existing deployments** → error is explicit, names both
  settings, and only triggers when preview is enabled without location.

## Migration Plan

1. Ship default off. Existing deployments rebuild to byte-identical output apart
   from the new optional field's absence.
2. To opt in: set `include_location = true` and `location_preview = true` (or the
   flags), rebuild so pages and index advertise the setting, then reload the site.
3. Roll back by removing the flag and rebuilding; no state migration and no
   database change.

## Open Questions

- Exact popover size and intent delay are tunable without changing the spec or
  approach; settle them during implementation.
