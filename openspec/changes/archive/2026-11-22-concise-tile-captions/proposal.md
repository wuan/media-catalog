# Proposal

## Why

Catalog tiles currently use two lines for a filename and a full `YYYY-MM-DD HH:MM:SS` timestamp. Visitors scanning month, event, and search grids need a shorter, consistent caption that identifies when an item was captured and, when known, the camera model that produced it. The generator deliberately strips camera metadata today, so showing a model requires a scoped privacy decision.

## What Changes

- Capture the camera model during probing: EXIF `Model` (`0x0110`) for JPEG images and ffprobe model tags for videos when present.
- Persist the model in build state and publish it in yearly shard JSON. GPS, location, serial numbers, and other camera details remain excluded.
- Render one-line tile captions as `YY-MM-DD HH:MM <model>`, falling back to `YY-MM-DD HH:MM` when no model is known.
- Apply the caption to month, event, and search grid tiles. The in-page viewer keeps its current filename title and full timestamp.
- Remove the visible filename from tile captions while keeping it in thumbnail alternative text and the viewer's title/link labels.
- Keep canonical timestamps as `YYYY-MM-DD HH:MM:SS` in state and JSON; the short form is display-only.
- **BREAKING** Advance the state and catalog JSON schema versions. Existing published snapshots must be rebuilt, and unchanged items need a one-time metadata refresh to populate the new model field.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `media-inventory`: capture and publish the camera model while continuing to exclude GPS, location, and serial metadata.
- `static-catalog`: present one-line tile captions on month, event, and search grids, and include the model in shard records.

## Impact

- Rust modules: `probe`, `state`, `model`, `json`, `render`, and the build pipeline in `lib`.
- Templates and assets: `templates/tiles.html`, `assets/search.js`, `assets/site.css`.
- Tests and snapshots: render, JSON, server, probe, state, and JavaScript checks.
- Specs: `media-inventory` and `static-catalog`.
- Operations: schema migration and one-time metadata refresh; published catalogs require a rebuild before the updated server accepts them.
