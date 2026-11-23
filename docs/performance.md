# Performance verification

Measurements taken on macOS (Apple silicon) with the release binary. Timings
come from the run summary and `/usr/bin/time -l`; RSS is the process maximum
resident set size.

## Real archive subset

Source: a copied subset of `MEDIA_CATALOG_DATA` (90 files, 489 MB, 80 images and
6 videos, plus 4 files outside the `YYYY/MM` hierarchy that are correctly
skipped). Build into separate temporary data and state roots:

```sh
target/release/media-catalog \
  --media-root "$REAL/media" \
  --data-root "$REAL/data" \
  --state-root "$REAL/state" \
  --jobs 6 \
  --quiet
```

| Run | Items | Result | Wall time | Max RSS |
| --- | --- | --- | --- | --- |
| Initial | 86 cataloged | 86 probed, 86 thumbnails generated | 5.9 s | 715 MB |
| Incremental | 86 | 0 probed, 0 generated, 86 skipped | 0.3 s | 10 MB |

- Original checksums were identical before and after the build.
- Generated JSON contained no GPS, latitude, longitude, make, or serial fields; the
  camera model and lens settings are published, and coordinates only with
  `--include-location`.
- Thumbnail, media, and JSON URLs resolved under `/thumbs/`, `/media/`, and
  `/catalog/` when served by `media-catalog-server`, and the dynamic index, year,
  and month pages rendered from the published JSON (200 responses for pages,
  JSON, embedded assets, a poster thumbnail, and an original video).
- The running server picked up the published catalog without a restart.

## Synthetic 75k-item tree

Generated with `cargo run --release --example gen_tree -- <root> 75000` and run
with `--jobs 8`:

| Run | Items | Work | Wall time | Max RSS |
| --- | --- | --- | --- | --- |
| Initial | 75,000 | 75,000 probed and thumbnailed | 440.0 s | 250 MB |
| Incremental (+100 new) | 75,100 | 100 probed/thumbnailed; 75,000 skipped | 2.7 s | 316 MB |

Initial output was 352 MB of thumbnails plus JSON; the state database was
17 MB. `--jobs` bounds decoding memory; the real subset used 6 workers to cap RSS
while decoding 12-megapixel photos.

## Resume after interruption

A 3,000-item tree was interrupted with `SIGTERM` after 2 seconds. Rerunning the
same command completed with:

```text
Scanned: 3000 items
New: 0, changed: 0, unchanged: 3000
Probed: 0
Thumbnails: 2696 generated, 304 skipped, 0 failed, 0 unsupported
Elapsed: 15.9 s
```

304 thumbnails had already been written and recorded before the interruption;
the rerun generated only the remaining 2696 and skipped the completed work.

## Camera metadata and location

Measured with a synthetic 5,000-item tree of identical 64x48 JPEGs carrying
EXIF aperture f/1.8, exposure 1/125 s, ISO 100, focal length 26 mm, and GPS
coordinates. Built with the release binary and `--jobs 8`:

| Run | Work | Wall time | Max RSS | Catalog JSON | State DB |
| --- | --- | --- | --- | --- | --- |
| Initial, location off | 5,000 probed and thumbnailed | 25.1 s | 84 MB | 2.4 MB | 2.0 MB |
| Same state, location on | 0 probed, 5,000 thumbnails skipped | ~0.1 s | 37 MB | 2.6 MB | 2.0 MB |

- Shard records measure about 499 bytes per item with lens settings and without
  coordinates. Enabling location publication adds about 52 bytes per item
  (2,493,006 -> 2,753,006 bytes for 5,000 items).
- The location toggle republishes from cached state; it does not re-probe.
- The server reported about 13 MB RSS after loading the 5,000-item catalog and
  rendering a month page.

For a 75k-item archive, the per-item measurements extrapolate to roughly 37 MB
of shard JSON with lens settings and about 4 MB more when coordinates are
published. Path and thumbnail URL length dominate the per-item size, so a
deeper tree will exceed these figures. Thumbnail bytes and decode memory still
dominate the generator's peak RSS, as in the measurements above.
