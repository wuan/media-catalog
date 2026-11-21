# media-catalog

Generate and serve a fast catalog for a photo/video archive laid out as
`YYYY/MM`. The generator scans the archive read-only, extracts safe metadata,
produces one WebP thumbnail per item, and publishes sharded JSON. The Rust server
is the only runtime: it renders the browsable catalog pages from that JSON, serves
the embedded search shell and assets, and streams originals from the media root.

The catalog becomes usable as soon as `catalog/index.json` exists, even while
thumbnails are still being generated.

## Requirements

- Rust (edition 2021 toolchain)
- `ffmpeg` and `ffprobe` on `PATH` for video probing and poster extraction.
  The build fails before scanning with a clear message when either is missing.
- The media root must already exist. Originals are never modified or copied.

## Build and test

```sh
cargo build --release
cargo test
cargo fmt --check
cargo clippy -- -D warnings
```

The binaries are `target/release/media-catalog` (generator) and
`target/release/media-catalog-server` (runtime). `--help` lists every flag.
Performance measurements for a real-archive subset and a synthetic 75k-item tree
are recorded in [`docs/performance.md`](docs/performance.md).

## Quick start

Organize the originals by folder; the folder path is the authoritative grouping:

```text
media/
  2023/12/IMG_0001.jpg
  2024/05/event/clip.mp4
```

Run the generator:

```sh
cargo run --release --bin media-catalog -- \
  --media-root ./media \
  --catalog-root ./catalog \
  --jobs 4
```

The catalog root owns published catalog data and build metadata:

```text
catalog/
  public/
    catalog/index.json     catalog commit point
    catalog/YYYY.json      per-year item shards
    thumbs/...             generated thumbnails
  state.sqlite             build state
  build.lock               concurrent-run guard
```

No HTML pages or browser assets are written. Start the server:

```sh
cargo run --release --bin media-catalog-server -- \
  --catalog-root ./catalog \
  --media-root ./media
```

It listens on `127.0.0.1:8080` by default:

- `/` → root years page rendered from JSON
- `/{year}/`, `/{year}/{month}/` → rendered year and month pages
- `/search.html`, `/assets/*` → embedded search shell, CSS, and JavaScript
- `/catalog/*` → published JSON
- `/thumbs/*` → generated thumbnails
- `/media/*` → read-only originals
- `/healthz` → catalog and media readiness

The server may start before any catalog exists. It serves the embedded shell and
a bootstrap page at `/`, then picks up the published catalog without a restart.
Later generator runs process only new or changed files.

## Configuration

CLI flags override an optional TOML file. If `--config` is omitted, the generator
reads `./media-catalog.toml` when that file exists.

| Flag | Default | Description |
| --- | --- | --- |
| `--media-root <path>` | required | Root of the read-only `YYYY/MM` archive. |
| `--catalog-root <path>` | required | Root for `public/`, `state.sqlite`, and `build.lock`. |
| `--media-url-prefix <path>` | `/media` | URL prefix for original media links. |
| `--thumb-url-prefix <path>` | `/thumbs` | URL prefix for thumbnail links. |
| `--jobs <n>` | `min(4, CPU cores)` | Parallel thumbnail/probe workers. |
| `--force` | off | Regenerate thumbnails even for unchanged fingerprints. |
| `--prune` | off | Delete thumbnails with no catalog item. |
| `--dry-run` | off | Scan and report planned work; write nothing. |
| `--quiet` | off | Suppress progress reporting on stderr. |
| `--allow-mount-drop` | off | Confirm deletions after a sharp item-count drop. |
| `--config <path>` | `./media-catalog.toml` if present | TOML configuration file. |

Example `media-catalog.toml`:

```toml
media_root = "/mnt/nas/media"
catalog_root = "./catalog"
media_url_prefix = "/media"
thumb_url_prefix = "/thumbs"
jobs = 4
```

URL prefixes are normalized to a leading slash and no trailing slash. The public
root is always `<catalog-root>/public`, and build state lives directly in the
catalog root outside the published tree.

The server accepts `--catalog-root`, `--media-root`, `--bind`, and
`--max-streams`. The access token is read from `MEDIA_CATALOG_TOKEN`, never from
the command line.

## Progress output

Progress messages go to stderr, keeping the final run summary on stdout easy to
capture. Each pipeline phase prints a start and finish line. Probing and
thumbnail generation show a live bar on a terminal; when stderr is redirected to
a file or cron log, they log a counter at roughly ten-percent intervals instead.
Use `--quiet` to suppress all progress output.

The state database and the `build.lock` file sit beside each other in the
catalog root. The lock is held for the whole run; a second build exits with a
clear message and writes nothing.

## Supported formats

Images:

- `jpg`, `jpeg` (JPEG, including EXIF orientation and capture time)
- `png`
- `webp`
- `heic`, `heif` are cataloged in v1 but reported as unsupported and shown with
  a placeholder tile. The decoder boundary allows HEIC support later without
  changing the state schema or renderer.

Videos (probed and poster-framed through ffmpeg/ffprobe):

- `mp4`, `mov`, `m4v`, `hevc`, `mkv`, `avi`, `webm`

Hidden files and directories, sidecars such as `.xmp`/`.aae`, unsupported
extensions, and files outside `YYYY/MM` are skipped. HEIC/HEIF and videos are
still cataloged when they cannot be decoded; individual failures never abort the
whole build.

## Metadata and privacy

For each item the catalog records only:

- width and height (upright)
- capture timestamp (`YYYY-MM-DD HH:MM:SS`, naive local wall clock)
- video duration in milliseconds
- media kind, relative path, thumbnail availability

JPEG capture time comes from EXIF `DateTimeOriginal`; when it is absent, the
file modification time is used. Videos use ffprobe's `creation_time` when
present and otherwise the modification time. EXIF timestamps carry no timezone,
so they are stored as naive local strings for deterministic, unambiguous sorting.

GPS coordinates, camera make/model, and other private EXIF fields are never
extracted or serialized into JSON or rendered pages. Thumbnails are re-encoded
from pixels and therefore carry no source metadata.

## Thumbnails

Every item gets one 512×512 WebP thumbnail at quality 80:

- EXIF orientation (photos) and ffmpeg autorotation (videos) are applied first.
- The center square is cropped and resized to 512×512 without distortion.
- Files mirror the source path and append `.webp` instead of replacing the
  extension:

  ```text
  original:  /media/2024/05/event/IMG_0001.jpg
  thumbnail: /thumbs/2024/05/event/IMG_0001.jpg.webp?v=<version>
  ```

- The version is derived from the source size and modification time and changes
  only when the source changes, giving stable URLs and cheap cache invalidation.
- Writes use a temporary file in the destination directory followed by a rename,
  so visitors never see partial thumbnails.
- Video posters use a frame at `min(1 second, 10% of duration)`, falling back to
  the first frame for very short or unseekable videos.
- Failed thumbnails are recorded and retried on the next run even when the
  fingerprint is unchanged. Unsupported formats keep a placeholder tile.

`--force` regenerates every thumbnail regardless of fingerprints.

Versioned thumbnail URLs are safe to cache for a long time. The server serves
them with an immutable cache policy.

## Published output

```text
public/
  catalog/index.json
  catalog/YYYY.json
  thumbs/...
```

`catalog/index.json` is the commit point. The generator writes every referenced
year shard first, replaces the index last, and then removes stale shards. An
interrupted publication therefore leaves the previous complete catalog in place.

The server keeps an immutable in-memory snapshot loaded from the index and its
shards. It checks the index metadata on each catalog request and reloads when it
changes. A reload that cannot read every referenced shard is discarded, so the
previous snapshot keeps serving and visitors never see a partial catalog.

Catalog pages are rendered per request with Askama:

- root years index at `/`
- months for a year at `/{year}/`
- lazy-loaded item grid at `/{year}/{month}/`

Month grids render every item with:

- `loading="lazy"` and `decoding="async"` thumbnails
- capture date and filename
- a `Video · duration` marker for videos
- a direct link to the original under `/media/`
- a neutral placeholder when no thumbnail exists

Pages work without JavaScript. All navigation is root-relative and all
thumbnail/media URLs are root-relative and percent-encoded per path segment
(spaces, Unicode, `#`, `%`). Askama escapes filenames and metadata, and every
thumbnail has non-empty alternative text. No per-item HTML pages are emitted.

Rendered pages carry an ETag derived from the catalog snapshot and the route, so
a matching `If-None-Match` returns `304`. Embedded shell assets carry a content
validator. Catalog JSON and pages revalidate (`no-cache`); versioned thumbnails
are immutable; originals use `private, no-cache`.

## Missing catalog and readiness

The generator and server have separate readiness:

- The server requires the media root and starts without `public/` or catalog JSON.
- `/`, `/search.html`, and `/assets/*` remain available before publication.
- `/` shows a bootstrap page stating that the catalog is not available yet.
- `/{year}/` and `/{year}/{month}/` return `503` before publication and `404` for
  unknown years or months.
- `/catalog/*` returns `404` until JSON exists.
- `/healthz` reports catalog readiness separately from media readiness.

When the generator publishes catalog JSON, pages and search data become available
without restarting the server.

## JSON catalog and search

`catalog/index.json` lists years, counts, months, and shard paths:

```json
{
  "schema_version": 1,
  "total_items": 3,
  "years": [
    {
      "year": 2024,
      "count": 2,
      "months": [{ "month": 5, "count": 2 }],
      "shard": "catalog/2024.json"
    }
  ]
}
```

Each `catalog/YYYY.json` shard contains ordered item records:

```json
{
  "schema_version": 1,
  "year": 2024,
  "items": [
    {
      "path": "2024/05/a.jpg",
      "month": 5,
      "kind": "image",
      "captured_at": "2024-05-01 10:11:12",
      "width": 4032,
      "height": 3024,
      "duration_ms": null,
      "thumb_url": "/thumbs/2024/05/a.jpg.webp?v=...",
      "media_url": "/media/2024/05/a.jpg",
      "thumb_available": true
    }
  ]
}
```

Search (`search.html` + `assets/search.js`, both embedded in the server binary)
loads the index and all year shards in parallel, then filters by text, type
(`image`/`video`), year, and month. Filters are reflected in the URL query
(`q`, `kind`, `year`, `month`) for shareable links and restored on page load.
Without JavaScript, the page explains the limitation and links to the browsable
year hierarchy. If catalog JSON cannot be loaded yet, the search page shows a
clear catalog-unavailable message.

JSON and rendered pages contain no build timestamps, so two runs over an
unchanged archive produce byte-identical output.

## Incremental builds, resume, and cleanup

The fingerprint is `size:mtime` (nanosecond modification time), stored in SQLite
with cached probe metadata and thumbnail status.

On each run:

1. Scan the media tree and diff fingerprints.
2. Probe only new/changed items, plus unchanged rows missing cached metadata.
3. Thumbnail only new/changed items, retries, missing files, and `--force` work.
4. Publish JSON shards, then `catalog/index.json`, then remove stale shards.
5. Remove state rows for deleted items (unless blocked by the mount guard).
6. Optionally prune orphan thumbnails under `public/thumbs/`.

Unchanged sources are not reopened for probing or thumbnailing. An interrupted
build leaves completed thumbnails recorded and in place; rerunning resumes the
unfinished work and retries failed items. Deleted items disappear from JSON and
therefore from server-rendered pages immediately.

### Prune safety and mount drops

Pruning is opt-in with `--prune` and is confined to `public/thumbs/`. It removes
files that do not belong to any current catalog item and cleans up empty
directories; no file outside the thumbnail tree is touched.

To protect against a stale or missing NAS mount, a run warns when the current
item count is less than half of the previous count (once the archive has at least
four rows). In that case the build skips prune and state cleanup, publishes the
current JSON view, and exits nonzero. Confirm real deletions with
`--allow-mount-drop` before pruning and state cleanup proceed.

## Deployment

The Rust server is the only supported runtime. Static-file deployment through
nginx or an rsync/snapshot workflow is no longer supported because catalog pages
and shell assets are rendered or embedded by the server.

```sh
cargo build --release --bin media-catalog-server

export MEDIA_CATALOG_TOKEN='choose-a-long-random-value' # optional
./target/release/media-catalog-server \
  --catalog-root "$HOME/.media-catalog" \
  --media-root /Volumes/media/photos \
  --bind 127.0.0.1:8080
```

It streams `/media/` with byte ranges and conditional requests, serves catalog
pages from JSON, and exposes `/healthz`. It binds to loopback by default, reads
the access token from `MEDIA_CATALOG_TOKEN`, and never writes to either root.

Keep `public/` and `state.sqlite` on local disk; SQLite WAL is not supported on
network filesystems. The media root can stay on the NAS. Systemd and launchd
examples live in [`docs/server.md`](docs/server.md).

A TLS-terminating reverse proxy may sit in front of the server; it must pass
`Authorization`, `Range`, and `If-Range` and avoid buffering `/media/`. The
server itself speaks plain HTTP.

### Upgrading from the static web tree

Older builds may have left these obsolete files in `public/`:

```text
public/index.html
public/YYYY/index.html
public/YYYY/MM/index.html
public/search.html
public/assets/
```

The new server ignores them and renders pages and assets itself. They are not
needed and can be removed manually after upgrading:

```sh
find public -type f -name index.html -delete
rm -f public/search.html
rm -rf public/assets
```

Do not remove `public/catalog/` or `public/thumbs/`. Keeping the previous binary
and generator is a straightforward rollback; the JSON/thumbnail tree stays
compatible.

### Initial build

The first complete build reads the entire archive and can take hours for a large
NAS collection. Use a modest `--jobs` value (for example 4) to bound memory on
large panoramas and avoid hammering the NAS. If the process is interrupted, rerun
the same command; completed thumbnails are skipped. Later incremental runs
process only new or changed files and are much faster.

## Run summary and exit status

The summary reports scanned counts, new/changed/unchanged items, probes,
thumbnails generated/skipped/failed/unsupported, JSON files written, deleted
rows, pruned files, per-item errors, and elapsed time. Exit status is nonzero for
configuration errors, any per-item failure, or a blocked mount-drop guard.
