# Performance verification

Measurements taken on macOS (Apple silicon) with the release binary. Timings
come from the run summary and `/usr/bin/time -l`; RSS is the process maximum
resident set size.

## Real archive subset

Source: a copied subset of `MEDIA_CATALOG_DATA` (90 files, 489 MB, 80 images and
6 videos, plus 4 files outside the `YYYY/MM` hierarchy that are correctly
skipped). Build into a separate temporary catalog root:

```sh
target/release/media-catalog \
  --media-root "$REAL/media" \
  --catalog-root "$REAL/catalog" \
  --jobs 6 \
  --quiet
```

| Run | Items | Result | Wall time | Max RSS |
| --- | --- | --- | --- | --- |
| Initial | 86 cataloged | 86 probed, 86 thumbnails generated | 5.9 s | 715 MB |
| Incremental | 86 | 0 probed, 0 generated, 86 skipped | 0.3 s | 10 MB |

- Original checksums were identical before and after the build.
- Generated JSON contained no GPS, latitude, longitude, camera, make, or model
  fields.
- Thumbnail and media URLs resolved under `/thumbs/` and `/media/` when the tree
  was served locally over HTTP (200 responses for HTML, JSON, assets, a poster
  thumbnail, and an original video).
- `rsync -a --delete` reproduced the public tree exactly and removed a stale
  page mirrored from the source.

## Synthetic 75k-item tree

Generated with `cargo run --release --example gen_tree -- <root> 75000` and run
with `--jobs 8`:

| Run | Items | Work | Wall time | Max RSS |
| --- | --- | --- | --- | --- |
| Initial | 75,000 | 75,000 probed and thumbnailed | 440.0 s | 250 MB |
| Incremental (+100 new) | 75,100 | 100 probed/thumbnailed; 75,000 skipped | 2.7 s | 316 MB |

Initial output was 352 MB of thumbnails plus HTML/JSON; the state database was
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
