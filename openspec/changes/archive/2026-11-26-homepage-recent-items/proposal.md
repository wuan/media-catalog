# Proposal

## Why

The root page is a navigation index: Events and Years, nothing visual. A returning visitor has to guess a year and month before seeing a single photo, even though the newest items are exactly what the archive is usually opened for. Surfacing the most recent items on `/` gives the catalog an immediate entry point and makes the viewer reachable in one click.

## What Changes

- Add a third root-page section, `Recent`, after Events and Years, showing the 24 newest catalog items as tiles in a six-column grid: four rows at desktop width, reflowing to three and two columns on narrower screens.
- Select items at render time from the loaded catalog snapshot: global descending capture timestamp, descending relative path as the deterministic tie-break. Images and videos are both included; videos keep the `Video · duration` badge.
- Render the strip with the existing tile markup, so captions, alt text, thumbnails, placeholders, and original links match month and event pages.
- Load `viewer.js` on `/` and set the location-preview body attribute so the strip behaves as an in-place album with deep links and the optional map preview.
- Show fewer than 24 tiles when the catalog holds fewer items; hide the section entirely when the catalog is empty. The bootstrap page before publication is unchanged.
- No published JSON, state schema, generator, or configuration change: the selection is derived from data the server already loads.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `media-server`: the root page requirement now includes a recent-items strip derived from the published catalog snapshot.
- `media-viewer`: the album scope now includes tiles rendered in the root page's recent section.

## Impact

- `src/render.rs`: `IndexTemplate` gains recent tiles; `render_index` sorts and selects them.
- `templates/index.html`: new section, tiles include, viewer script tag, location-preview attribute.
- `assets/site.css`: a recent-grid column rule and breakpoints.
- Tests and snapshots for the root page, plus end-to-end root assertions.
- `README.md`: root page route description.
- Untouched: generator, JSON schema, state database, thumbnail pipeline, configuration.
