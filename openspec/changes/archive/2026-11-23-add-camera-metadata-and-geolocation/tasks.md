# Tasks

## 1. Dependency and EXIF extraction

- [x] 1.1 Add `kamadak-exif = "0.6"` to `Cargo.toml` and confirm `cargo check` resolves it with the existing toolchain
- [x] 1.2 Replace the hand-written TIFF/JPEG reader in `src/probe.rs` with `exif::Reader::read_from_container`, preserving orientation (1..=8), normalized model whitespace, and the `YYYY-MM-DD HH:MM:SS` capture timestamp; verify existing probe tests and snapshots still pass
- [x] 1.3 Extract aperture, exposure time, ISO, and focal length from raw EXIF `Value` variants in `ProbeResult`; verify unit tests cover a full set, partial sets, and zero-denominator/non-positive rationals
- [x] 1.4 Extract GPS latitude/longitude with N/S/E/W hemisphere handling, six-decimal rounding, and rejection of missing, out-of-range, zero-denominator, and `0,0` coordinates; verify unit tests cover each case
- [x] 1.5 Parse Apple QuickTime ISO 6709 location tags and a generic `location` tag from ffprobe output; verify a unit test covers signed coordinates and rejects malformed values
- [x] 1.6 Add a metadata-only image probe for HEIC/HEIF that reads container EXIF and optional `PixelXDimension`/`PixelYDimension` without touching the raster decoder; verify with a generated HEIC fixture when ffmpeg supports it, otherwise a documented skip plus a synthetic EXIF test

## 2. State, configuration, and pipeline

- [x] 2.1 Add the six metadata columns to `src/state.rs`, bump `SCHEMA_VERSION` to 3, and migrate v2 databases by adding columns and clearing cached probe fields once while keeping thumbnail columns; verify migration tests assert thumbnails survive and probe fields are cleared
- [x] 2.2 Extend `ItemRecord`, `set_probe`, and `load_items` for aperture, exposure numerator/denominator, ISO, focal length, latitude, and longitude; verify a round-trip test reloads every value
- [x] 2.3 Add `include_location` to `Cli`/`Config` with flag-over-file precedence and a default of off, and keep the server accepting and ignoring the key; verify config tests cover flag, TOML, precedence, default, and invalid value
- [x] 2.4 Route non-decodable images into metadata-only probe work in `src/lib.rs` while keeping `ThumbStatus::Unsupported`; verify an end-to-end test catalogs a HEIC item with metadata and no thumbnail without a probe failure
- [x] 2.5 Extend `CatalogItemInput` and `ItemView` with the new optional fields and apply the location publication gate when building catalog inputs; verify model tests cover presence, absence, and the disabled gate

## 3. Published catalog output

- [x] 3.1 Add the optional lens and location fields to `ShardItem`, serializing each only when present and only publishing coordinates when location is enabled; verify JSON tests assert the fields appear and disappear correctly
- [x] 3.2 Implement deterministic serialization for exposure fractions, aperture, focal length, and coordinates; verify two renders of the same catalog are byte-identical
- [x] 3.3 Update `tile_from_item` in `src/render.rs` to emit `data-f-number`, `data-exposure`, `data-iso`, `data-focal`, and conditional `data-lat`/`data-lon`; verify render tests assert attributes appear only when known or enabled
- [x] 3.4 Replace the private-metadata test in `src/json.rs` with paired assertions: no latitude/longitude/location fields when disabled, and coordinates when enabled; verify both directions pass
- [x] 3.5 Update README JSON examples and `docs/performance.md` for the new fields, the opt-in flag, and the privacy contract; verify the README example still parses in its existing test and the documented command runs as written
- [x] 3.6 Refresh JSON snapshots deliberately and review the diff for unintended changes to existing fields; verify `cargo test` passes with the new snapshots

## 4. Viewer and search

- [x] 4.1 Parse `data-f-number`, `data-exposure`, `data-iso`, `data-focal`, and `data-lat`/`data-lon` in `tileFromElement`; verify viewer JS checks expose the parsed values
- [x] 4.2 Add formatting helpers for `26 mm · f/1.8 · 1/125 s · ISO 100`, including reduced fractions below one second, seconds with one decimal at or above one second, whole-number trimming, and omission of unknown values; verify viewer JS checks cover every formatting scenario
- [x] 4.3 Render a second metadata line inside `#viewer-caption` and show the location anchor only when coordinates are present; verify the caption line remains the accessible name and the metadata line is read as description
- [x] 4.4 Build the OpenStreetMap URL and label helpers with coordinate markers, a new tab, `noopener noreferrer`, and `referrerpolicy="no-referrer"`; verify viewer JS checks assert the URL and label and that no map request is made before activation
- [x] 4.5 Add styling for the metadata and location lines in `assets/site.css`; verify the viewer remains usable at phone width and with long metadata
- [x] 4.6 Set the same dataset attributes from shard records in `assets/search.js` and gate coordinates; verify search JS checks cover present and absent metadata
- [x] 4.7 Update viewer and search integration tests to cover a month tile, an event tile, and a search tile carrying metadata; verify the rendered markup contains the expected attributes and no filename in the caption

## 5. Integration verification

- [x] 5.1 Run an end-to-end build over a fixture tree with JPEG EXIF/GPS, HEIC metadata, and a video location tag, first with location off and then on; verify the published JSON and rendered pages match each specification scenario
- [x] 5.2 Verify the incremental upgrade path: migrate a v2 database, confirm the first run probes all items but skips valid thumbnails, then confirm the next unchanged run probes nothing
- [x] 5.3 Measure output growth and memory on a synthetic tree: record shard bytes, state database size, server RSS, and generator peak RSS; verify the measured values are close to the design estimates and update the numbers in `docs/performance.md`
- [x] 5.4 Run `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`, the JS view/search checks, and `openspec validate add-camera-metadata-and-geolocation --strict`; verify all pass

## Workflow follow-up

- Archive the change after review and verification are complete.
- Confirm the archived specs reflect the opt-in location behavior before publishing a release.
