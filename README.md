# media-catalog

Generate a fast, deployable static catalog for a photo/video archive laid out as
`YYYY/MM`. The generator scans the archive read-only, extracts safe metadata,
produces one WebP thumbnail per item, renders browsable HTML, and emits sharded
JSON for client-side search. Build state lives in SQLite so reruns only process
new or changed items.

By default, the archive and the web tree are served by the same web server:

- `/` → generated `public/` tree
- `/media/` → read-only NAS mount (the originals)
- `/thumbs/` → generated thumbnails inside `public/`

When the web host cannot mount the NAS share, the optional
`media-catalog-server` binary serves the generated tree and streams originals
from a host that can read the media root. See [docs/server.md](docs/server.md).

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

The release binary is `target/release/media-catalog`. `--help` lists every flag.
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
cargo run --release -- \
  --media-root ./media \
  --catalog-root ./catalog \
  --jobs 4
```

The catalog root owns the deployable tree and build metadata:

```text
catalog/
  public/                deployable web tree
  state.sqlite           build state
  build.lock             concurrent-run guard
```

The first run creates `public/`, thumbnails, HTML, and JSON. Later runs process
only new or changed files:

```sh
cargo run --release -- --media-root ./media --catalog-root ./catalog --jobs 4
```

## Configuration

CLI flags override an optional TOML file. If `--config` is omitted, the
generator reads `./media-catalog.toml` when that file exists.

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
catalog root, outside the deployable tree.

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
extracted or serialized into HTML or JSON. Thumbnails are re-encoded from pixels
and therefore carry no source metadata.

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

## Generated web tree

```text
public/
  index.html              root years page
  YYYY/index.html         months for one year
  YYYY/MM/index.html      item grid for one month
  search.html             client-side search and filters
  assets/site.css
  assets/search.js
  catalog/index.json
  catalog/YYYY.json
  thumbs/...
```

Month grids render every item with:

- `loading="lazy"` and `decoding="async"` thumbnails
- capture date and filename
- a `Video · duration` marker for videos
- a direct link to the original under `/media/`
- a neutral placeholder when no thumbnail exists

Pages render without JavaScript. All navigation is root-relative and all
thumbnail/media URLs are root-relative and percent-encoded per path segment
(spaces, Unicode, `#`, `%`). Askama escapes filenames and metadata, and every
thumbnail has non-empty alternative text. No per-item HTML pages are emitted.

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

Search (`search.html` + `assets/search.js`) loads the index and all year shards
in parallel, then filters by text, type (`image`/`video`), year, and month. Filters
are reflected in the URL query (`q`, `kind`, `year`, `month`) for shareable links
and restored on page load. Without JavaScript, the page explains the limitation
and links to the browsable year hierarchy.

HTML and JSON contain no build timestamps, so two runs over an unchanged archive
produce byte-identical output.

## Incremental builds, resume, and cleanup

The fingerprint is `size:mtime` (nanosecond modification time), stored in SQLite
with cached probe metadata and thumbnail status.

On each run:

1. Scan the media tree and diff fingerprints.
2. Probe only new/changed items, plus unchanged rows missing cached metadata.
3. Thumbnail only new/changed items, retries, missing files, and `--force` work.
4. Regenerate all HTML and JSON from the current catalog.
5. Remove state rows for deleted items (unless blocked by the mount guard).
6. Optionally prune orphan thumbnails under `public/thumbs/`.

Unchanged sources are not reopened for probing or thumbnailing. An interrupted
build leaves completed thumbnails recorded and in place; rerunning resumes the
unfinished work and retries failed items. Deleted items disappear from pages and
JSON immediately.

### Prune safety and mount drops

Pruning is opt-in with `--prune` and is confined to `public/thumbs/`. It removes
files that do not belong to any current catalog item and cleans up empty
directories; no file outside the thumbnail tree is touched.

To protect against a stale or missing NAS mount, a run warns when the current
item count is less than half of the previous count (once the archive has at least
four rows). In that case the build skips prune and state cleanup, still renders
the current view, and exits nonzero. Confirm real deletions with
`--allow-mount-drop` before pruning and state cleanup proceed.

## Deployment

### Web server (nginx)

Serve the generated tree at `/` and the NAS originals read-only at `/media/`:

```nginx
server {
    listen 80;
    server_name media.example.lan;
    root /srv/media-catalog/public;

    location /media/ {
        alias /mnt/nas/media/;
        autoindex off;
    }

    location /thumbs/ {
        # Thumbnail URLs carry ?v=<fingerprint>, so long-lived immutable caching is safe.
        add_header Cache-Control "public, max-age=2592000, immutable";
        try_files $uri =404;
    }

    location ~* \.html$ {
        add_header Cache-Control "no-cache";
    }

    location /catalog/ {
        add_header Cache-Control "no-cache";
    }
}
```

HTML and JSON should revalidate; versioned thumbnails can be cached for a long
time because a changed source changes the URL.

### Runtime server (Rust)

If nginx cannot alias the NAS mount, build and run the optional server instead:

```sh
cargo build --release --bin media-catalog-server

export MEDIA_CATALOG_TOKEN='choose-a-long-random-value' # optional
./target/release/media-catalog-server \
  --catalog-root "$HOME/.media-catalog" \
  --media-root /Volumes/media/photos \
  --bind 127.0.0.1:8080
```

It serves the catalog at `/`, streams `/media/` with byte ranges and conditional
requests, and exposes `/healthz`. It binds to loopback by default, reads the
access token from `MEDIA_CATALOG_TOKEN`, and never writes to either root.

Keep `public/` and `state.sqlite` on local disk; SQLite WAL is not supported on
network filesystems. The media root can stay on the NAS. A media-gateway and
reverse-proxy topology, systemd/launchd examples, TLS guidance, and the
browser-playback matrix are documented in [docs/server.md](docs/server.md).

### Snapshot or rsync deployment

Keep the previous `public/` snapshot for rollback. Deploy changes with:

```sh
rsync -a --delete public/ media@web:/srv/media-catalog/public/
```

`--delete` removes stale pages, shards, and pruned thumbnails on the server.

### Initial build

The first complete build reads the entire archive and can take hours for a large
NAS collection. Use a modest `--jobs` value (for example 4) to bound memory on
large panoramas and avoid hammering the NAS. If the process is interrupted, rerun
the same command; completed thumbnails are skipped. Later incremental runs
process only new or changed files and are much faster.

## Run summary and exit status

The summary reports scanned counts, new/changed/unchanged items, probes,
thumbnails generated/skipped/failed/unsupported, pages and JSON written, deleted
rows, pruned files, per-item errors, and elapsed time. Exit status is nonzero for
configuration errors, any per-item failure, or a blocked mount-drop guard.
