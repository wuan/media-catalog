# Proposal

## Why

The catalog already publishes the camera model and file size, but the viewer says
nothing about how a photo was taken or where it was taken. Aperture, exposure
time, ISO, focal length, and geocoordinates are present in most JPEG/HEIC EXIF
blocks and in some video tags, yet the generator deliberately discards them. That
leaves visitors without the context the archive already contains.

## What Changes

- Replace the hand-written JPEG/TIFF EXIF reader with `kamadak-exif`, which reads
  EXIF from JPEG, TIFF, HEIF, PNG, and WebP.
- Extract aperture (`FNumber`), exposure time (`ExposureTime`), ISO
  (`PhotographicSensitivity`), and focal length (`FocalLength`), in addition to
  the existing orientation, capture time, and camera model.
- Extract GPS latitude/longitude from JPEG/HEIC EXIF and from Apple QuickTime
  location tags on video.
- Publish lens settings in item records and viewer captions by default.
- Add opt-in geolocation publication through `--include-location` /
  `include_location = false` (default off). When disabled, no coordinates appear
  in generated HTML, JSON, or viewer attributes.
- Show a second viewer metadata line (`26 mm · f/1.8 · 1/125 s · ISO 100`) and,
  when coordinates are published, an OpenStreetMap link.
- Carry the values to the viewer through tile `data-*` attributes for month,
  event, and search grids.
- **BREAKING** (privacy contract): the existing guarantee that generated output
  contains no camera details other than the model is narrowed. Lens settings are
  now published by default, and coordinates are published when opted in.
- **BREAKING** (build state): the state schema moves to v3 and the first run
  after upgrade re-probes existing items once to fill the new fields.

Out of scope: map embeds, reverse geocoding, place names, altitude, serial
numbers, lens make/model, and thumbnail generation for HEIC.

## Capabilities

### New Capabilities

None. The work extends existing catalog behavior.

### Modified Capabilities

- `media-inventory`: replace the restricted metadata extraction rule, add lens
  fields, add opt-in GPS, and define metadata-only probing for HEIC.
- `static-catalog`: add lens and optional location fields to shard records and
  tile attributes.
- `media-viewer`: present the metadata line and the OpenStreetMap link, and keep
  the caption accessible.
- `configuration`: add the `include_location` setting and its precedence.
- `incremental-builds`: persist the new fields, migrate state to v3, and define
  the one-time re-probe while preserving thumbnails.

## Impact

- `Cargo.toml`: add `kamadak-exif = "0.6"`, removing the hand-rolled TIFF parser.
- `src/probe.rs`: crate-based EXIF extraction, GPS conversion, video location
  parsing, metadata-only probing for HEIC.
- `src/state.rs`: schema v3 columns and migration that clears cached probe fields
  once.
- `src/lib.rs`: probe routing for non-decodable images, new fields in catalog
  inputs and probe persistence.
- `src/model.rs`, `src/json.rs`, `src/render.rs`: item fields, shard records,
  tile attributes, optional location gating.
- `assets/viewer.js`, `assets/search.js`, `templates/tiles.html`: metadata display
  and location link.
- Tests and snapshots: add EXIF/GPS fixtures, update the private-metadata test to
  assert absence only when location is disabled, and extend the viewer JS checks.
- Docs: README, `docs/performance.md`, and the affected specs.
- Published JSON grows by roughly 137 bytes per fully tagged photo; the server
  snapshot grows about 15-20% in item memory. No new service or request path.
