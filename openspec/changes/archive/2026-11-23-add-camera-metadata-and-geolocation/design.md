# Design

## Context

See `proposal.md` for motivation. The relevant current state:

- `src/probe.rs` reads only a JPEG APP1 segment with a hand-written TIFF reader and keeps orientation, capture time, and model. `read_jpeg_exif` rejects anything that is not `FF D8`.
- `probe_image` opens every image through the `image` crate for dimensions, so HEIC fails before any thumbnail work. `src/lib.rs` excludes non-decodable extensions from probing and writes a placeholder tile.
- `src/state.rs` is schema v2. Cached probe fields are cleared by fingerprint changes and by the v1->v2 migration; thumbnails stay valid.
- `src/model.rs` / `src/json.rs` define `ItemView` and `ShardItem`, and `src/render.rs` turns items into tiles. The viewer reads tile `data-*` attributes, so metadata must reach both server-rendered and search-rendered tiles.
- `src/json.rs` asserts that `gps`, `location`, `latitude`, and `longitude` never appear in published output.

## Goals / Non-Goals

**Goals:**

- Read camera settings and coordinates from JPEG, HEIC, PNG, and WebP containers.
- Publish lens settings by default and coordinates only when opted in.
- Show settings and an OpenStreetMap link in the viewer without leaving the album page.
- Keep incremental builds fast after a single one-time re-probe.
- Keep output deterministic and backward compatible for older shards.

**Non-Goals:**

- Map embeds, reverse geocoding, place names, or altitude.
- Serial numbers, lens make/model, owner/artist fields.
- HEIC thumbnails or HEIC pixel decoding.
- A new runtime endpoint or a separate metadata service.
- Changing the `YYYY/MM` hierarchy, search filters, or event semantics.

## Decisions

### Use `kamadak-exif` 0.6.2

**Chosen:** add `kamadak-exif = "0.6"` (import `exif`) and call `Reader::read_from_container` on a `BufReader`. It supports TIFF, JPEG, HEIF, PNG, and WebP, is pure Rust, BSD-2-Clause, and has one tiny dependency.

**Alternatives:**

- Extend the hand-written TIFF reader for the new tags: smallest diff, but still JPEG-only and would need a second parser for HEIF/PNG/WebP.
- Use the `image` crate's EXIF support: does not cover HEIC metadata, and ties metadata to decoding.
- Use ffprobe for image metadata: inconsistent tags across formats and an external process per image.

**Behavior preservation:** read raw `Value` variants, not `display_value`. Keep the existing normalization for the model (trim and collapse whitespace) and the timestamp (`%Y:%m:%d %H:%M:%S` -> `%Y-%m-%d %H:%M:%S`), and keep orientation restricted to 1..=8. Snapshots should stay byte-identical for items that only have the old fields.

### Model camera settings as exact rationals and rounded floats

**Chosen:** probe and persist:

| Field | State column | JSON field | Display form |
| --- | --- | --- | --- |
| aperture | `f_number REAL` | `f_number` | `f/1.8` |
| exposure | `exposure_num`, `exposure_den INTEGER` | `exposure_time` string | `1/125 s` |
| ISO | `iso INTEGER` | `iso` | `ISO 100` |
| focal length | `focal_length_mm REAL` | `focal_length_mm` | `26 mm` |
| latitude | `latitude REAL` | `latitude` | `52.520008` |
| longitude | `longitude REAL` | `longitude` | `13.404954` |

Exposure stays exact as a numerator/denominator pair in state; JSON stores the reduced fraction string. Aperture and focal length are rounded to one decimal, coordinates to six decimals (~0.1 m), so repeated runs serialize identical bytes. Each field is optional and omitted when absent.

**Alternative:** store display strings for every field. Smaller JSON for exposure but loses numeric filtering and makes formatting decisions irreversible.

### Gate coordinates at publication, not extraction

Probe and persist coordinates whenever EXIF provides them; apply `include_location` only when building `CatalogItemInput`/`ShardItem` and tile attributes. This keeps a toggle cheap: enabling or disabling location republishes from state without reopening files. The state database is local build state, and the source media already contains the coordinates.

**Alternative:** skip extraction when disabled. Stronger at-rest privacy, but toggling on would force a second full re-probe or a probe-settings version marker.

### Extend `ProbeResult`, `CatalogItemInput`, `ItemView`, and `ShardItem` with optional fields

All new fields are `Option<...>` and serialized with `skip_serializing_if = "Option::is_none"`. `catalog_from_parts` and `to_item_view` tolerate older shards because serde ignores missing fields and defaults them to `None`. `JSON_SCHEMA_VERSION` stays `2`: the change is additive and follows the precedent of `size_bytes`.

**Alternative:** bump to schema 3. Rejected because it would reject older catalogs and force a coordinated client upgrade without a compatibility benefit.

### Add metadata to tiles as data attributes

`tile_from_item` emits `data-f-number`, `data-exposure`, `data-iso`, `data-focal`, and only when location is enabled `data-lat` / `data-lon`. `assets/search.js` sets the same dataset entries from the shard record. `tileFromElement` parses them; the viewer owns formatting.

This costs roughly 160 bytes per fully tagged tile (~80 KB on a 500-item month page) and avoids an extra fetch. The visible one-line tile caption is unchanged.

**Alternatives:**

- Have the viewer fetch `catalog/YYYY.json` on open: avoids page growth but adds a multi-megabyte request for one item's metadata.
- Add a separate per-item metadata file: more requests and more files to manage.

### Viewer presentation

Add a second line inside `#viewer-caption`:

```text
24-05-01 10:11 · iPhone 15 Pro · 3.4 MB
26 mm · f/1.8 · 1/125 s · ISO 100
Location 52.5200, 13.4050 -> OpenStreetMap
```

- Keep `#viewer-captured` as the dialog's accessible name; it stays the timestamp/model/size line.
- The metadata line is part of `aria-describedby`, so it is announced as description text.
- The location anchor uses `target="_blank"`, `rel="noopener noreferrer"`, and `referrerpolicy="no-referrer"`. Its accessible label identifies the coordinates, OpenStreetMap, and the new tab.
- Build the OSM URL in JS: `https://www.openstreetmap.org/?mlat=<lat>&mlon=<lon>#map=16/<lat>/<lon>`. No map request happens until activation.
- Show each known setting; omit unknown parts rather than printing empty separators. Videos usually have no lens settings, so only a location link may appear.

**Formatting rules:**

- Exposure `< 1 s`: reduced fraction from the stored rational, e.g. `1/125 s`, `3/10 s`.
- Exposure `>= 1 s`: seconds with up to one decimal, e.g. `4 s`, `2.5 s`.
- Aperture: `f/` plus a number with at most one decimal, trailing `.0` removed.
- Focal length: number with at most one decimal plus ` mm`, trailing `.0` removed.
- ISO: whole number.

### HEIC metadata-only probe

Split image probing into two paths:

- decodable images: current `probe_image` (EXIF + `image` dimensions)
- non-decodable images (HEIC/HEIF): new metadata-only probe that reads container EXIF, optionally takes `PixelXDimension`/`PixelYDimension` (orientation-adjusted) as dimensions, and never touches the `image` decoder

`src/lib.rs` includes these files in probe work, keeps `ThumbStatus::Unsupported`, and preserves the run-summary count. A probe failure falls back to the modification time and does not abort the build.

**Alternative:** leave HEIC metadata-blind. Rejected because it defeats the point of the crate choice for iPhone archives.

### Video coordinates from tags

Parse `com.apple.quicktime.location.ISO6709` and a generic `location` tag. ISO 6709 signs define the hemisphere; validate latitude +/-90 and longitude +/-180. Do not attempt lens settings or altitude from video.

### State schema v3 and one-time re-probe

Add the six columns and bump `SCHEMA_VERSION` to 3. The v2->v3 migration adds columns and clears `width`, `height`, `duration_ms`, `captured_at`, `model`, and the new fields once, leaving thumbnail columns and versions untouched. The existing probe filter already reprobes rows with missing `width` or `captured_at`, so the next run fills all items without regenerating thumbnails.

`Reuse metadata across location publication changes` is satisfied because coordinates are persisted regardless of the publication flag.

## Risks / Trade-offs

- **Crate output differs from the hand parser** -> Use raw `Value::Ascii` plus the existing normalization, keep the synthetic EXIF fixtures, and update snapshots deliberately rather than accepting incidental drift.
- **HEIC parsing cost** -> Metadata-only probe avoids pixel decoding; one file read per HEIC item, bounded and cached in state.
- **Coordinate privacy** -> Opt-in default off, rounded to six decimals, no coordinates in tile captions, and tests assert absence when disabled. Documentation and the privacy requirement change together.
- **Third-party request** -> The OSM link is click-only with `noreferrer`; no tiles or scripts load in the background.
- **HTML growth** -> ~160 bytes per tagged tile. If month pages become a concern, collapse the attributes into one packed `data-exif` value later; the viewer contract can stay the same.
- **Migration is one-way** -> A v3 database is rejected by older binaries. Rollback requires restoring a pre-upgrade state backup or deleting the state database to rebuild. Document this in the release notes.
- **Memory** -> Server `ItemView` grows ~15-20%; at 75k items that is roughly +6 MB on a ~36 MB snapshot. Generator peak RSS is dominated by decode workers and changes little. JSON grows ~137 bytes per fully tagged photo; at 75k that is ~10 MB on top of ~28 MB of shards. State grows ~3-4 MB on a 17 MB database.
- **Search page weight** -> The browser already loads every shard; the new fields add JSON text and a few properties per JS object, which is acceptable but measurable on large archives.

## Migration Plan

1. Add the dependency and extend the probe structs and state schema.
2. Ship with `include_location` off by default; existing deployments keep their privacy behavior.
3. First run after upgrade migrates the state database, re-probes all items once, and republishes shards. Thumbnails are reused.
4. Enable `--include-location` or `include_location = true` to publish coordinates; no re-probe is required.
5. Verify output with the existing snapshot and end-to-end tests, plus new EXIF/GPS fixtures.
6. Rollback: restore the pre-upgrade state database (or delete `state.sqlite` and rebuild) before running an older binary.

## Open Questions

None. Coordinate rounding precision and HEIC thumbnail support are intentionally fixed by this design; either can be revisited without changing the specs or task breakdown.
