# Proposal

## Why

The viewer shows published coordinates as a plain link, so checking where a photo
was taken means leaving the album. A small map preview on hover or focus would
answer that in place, but it makes a third-party request to OpenStreetMap that
today happens only on click. Visitors should get the preview only when the
deployment explicitly asks for it, so the existing privacy contract stays intact
for everyone else.

## What Changes

- Add an opt-in `location_preview` setting resolved from `--location-preview`
  first and the `location_preview` TOML key second, defaulting to off.
- Reject `location_preview = true` when location publication is disabled, naming
  both settings; there is nothing to preview without published coordinates.
- Emit the resolved setting into the static output: a `data-location-preview`
  attribute on generated month and event pages, and a `location_preview` field in
  `catalog/index.json` so the search page can learn it from the index it already
  loads.
- When enabled, hovering or focusing the coordinates reveals a small map preview
  using OpenStreetMap's official embed, after a short intent delay, with no
  request before that intent and at most one request per item.
- Keep the coordinates and their OpenStreetMap link unchanged in every case;
  the flag gates only the preview surface.
- Touch keeps today's behavior: tapping the link opens OpenStreetMap. No preview
  is invented for coarse pointers.
- Correct the README privacy section, which still claims geocoordinates are never
  extracted, to describe the opt-in publication and preview behavior.

Not breaking: default off reproduces today's output and requests exactly.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `configuration`: add the `location_preview` setting, its precedence and default,
  and the validation rule that ties it to `include_location`.
- `static-catalog`: emit the preview setting on month and event pages and in
  `catalog/index.json`, and have the search page propagate it to the viewer.
- `media-viewer`: make the OpenStreetMap requirement conditional on the setting,
  define the preview's intent trigger, request budget, accessibility, failure
  fallback, and keep the link behavior unconditional.

## Impact

- `src/config.rs`: new flag/key, resolution, validation, and tests.
- `src/render.rs` and `templates/month.html`, `templates/event.html`: preview
  attribute on `<body>`.
- `src/json.rs`: index field, omission when off, and privacy-test coverage.
- `assets/search.js`: propagate the index flag onto the page.
- `assets/viewer.js`, `assets/site.css`: popover, iframe lifecycle, intent
  handling, placement, and fallback.
- `tests/js/viewer_browser.mjs`: request-budget and preview assertions.
- `README.md`: flag table, TOML example, viewer and privacy sections.
- No Rust probe, scan, or thumbnail changes; no new runtime dependency.
