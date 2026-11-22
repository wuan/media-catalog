# media-catalog

Generate and serve a fast catalog for a photo/video archive laid out as
`YYYY/MM`. The generator scans the archive read-only, extracts safe metadata,
produces one WebP thumbnail per item, and publishes sharded JSON. The Rust server
is the only runtime: it renders the browsable catalog pages from that JSON, serves
the embedded search shell and assets, and streams originals from the media root.

The catalog becomes usable as soon as `catalog/index.json` exists, even while
thumbnails are still being generated. Authored events can additionally group
items across month boundaries without changing the `YYYY/MM` hierarchy.

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
  --data-root ./data \
  --state-root ./state \
  --jobs 4
```

The three roots each own one concern:

```text
media/
  2023/12/IMG_0001.jpg
  2024/05/event/clip.mp4

data/
  catalog/index.json     catalog commit point
  catalog/YYYY.json      per-year item shards
  thumbs/...             generated thumbnails

state/
  state.sqlite           build state
  build.lock             concurrent-run guard
```

No HTML pages or browser assets are written. Start the server:

```sh
cargo run --release --bin media-catalog-server -- \
  --data-root ./data \
  --media-root ./media
```

It listens on `127.0.0.1:8080` by default:

- `/` → root page rendered from JSON, with events above the year hierarchy
- `/events/`, `/events/{slug}/` → rendered event list and event pages
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

`media-catalog` and `media-catalog-server` read the same optional TOML file. CLI
flags override file values, and file values override built-in defaults where a
default exists. If `--config` is omitted, each binary reads
`./media-catalog.toml` when that file exists. Relative paths in the file resolve
against the process working directory, not the directory containing the file.

| Flag | Default | Description |
| --- | --- | --- |
| `--media-root <path>` | required | Root of the read-only `YYYY/MM` archive. |
| `--data-root <path>` | required | Root for published `catalog/` and `thumbs/`. |
| `--state-root <path>` | required (generator) | Root for `state.sqlite` and `build.lock`. |
| `--media-url-prefix <path>` | `/media` | URL prefix for original media links. |
| `--thumb-url-prefix <path>` | `/thumbs` | URL prefix for thumbnail links. |
| `--jobs <n>` | `min(4, CPU cores)` | Parallel thumbnail/probe workers. |
| `--force` | off | Regenerate thumbnails even for unchanged fingerprints. |
| `--prune` | off | Delete thumbnails with no catalog item. |
| `--dry-run` | off | Scan and report planned work; write nothing. |
| `--quiet` | off | Suppress progress reporting on stderr. |
| `--allow-mount-drop` | off | Confirm deletions after a sharp item-count drop. |
| `--bind <addr>` | `127.0.0.1:8080` | Server listen address; overrides `bind` in TOML. |
| `--max-streams <n>` | `8` | Server concurrent `/media/` streams; overrides `max_streams` in TOML. |
| `--config <path>` | `./media-catalog.toml` if present | Shared TOML configuration file. |
| `--events <path>` | none | Authored events content file; overrides `events_file` from TOML. |

Example `media-catalog.toml`:

```toml
media_root = "/mnt/nas/media"
data_root = "./data"
state_root = "./state"
events_file = "./events.toml"
media_url_prefix = "/media"
thumb_url_prefix = "/thumbs"
jobs = 4
bind = "127.0.0.1:8080"
max_streams = 8
```

Both binaries accept every key in this schema. The generator ignores `bind` and
`max_streams`; the server ignores `state_root`, `events_file`, URL prefixes,
`jobs`, and generator-only flags. Keys outside the schema are rejected.

URL prefixes are normalized to a leading slash and no trailing slash. Published
data is written directly under the data root, and build state lives in the state
root outside the published tree.

`events_file` is optional. Paths in the configuration file are resolved against
the current directory, like the storage roots. When neither `events_file` nor
`--events` is set, the catalog has no events; a configured path must exist and
be a file.

The server resolves `--data-root`, `--media-root`, `--bind`, and `--max-streams`
from the flag first, then the matching TOML key, then the built-in default where
one exists. Storage roots have no default. The access token is read from
`MEDIA_CATALOG_TOKEN`, never from the command line or the configuration file.

## Progress output

Progress messages go to stderr, keeping the final run summary on stdout easy to
capture. Each pipeline phase prints a start and finish line. Probing and
thumbnail generation show a live bar on a terminal; when stderr is redirected to
a file or cron log, they log a counter at roughly ten-percent intervals instead.
Use `--quiet` to suppress all progress output.

The state database and the `build.lock` file sit beside each other in the state
root. The lock is held for the whole run; a second build exits with a clear
message and writes nothing.

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
- camera model when the source provides one
- media kind, relative path, thumbnail availability

JPEG capture time comes from EXIF `DateTimeOriginal`; when it is absent, the
file modification time is used. Videos use ffprobe's `creation_time` when
present and otherwise the modification time. EXIF timestamps carry no timezone,
so they are stored as naive local strings for deterministic, unambiguous sorting.
The camera model comes from JPEG EXIF `Model` or common ffprobe model tags.

GPS coordinates, location data, serial numbers, and other private EXIF fields
are never extracted or serialized into JSON or rendered pages. The camera model
is published because it appears in tile captions. Thumbnails are re-encoded from
pixels and therefore carry no source metadata.

## Events

Events are authored content, kept in their own file and pointed to by
deployment configuration. Set `events_file` in `media-catalog.toml`, or pass
`--events <path>` to override it for a single run. The server never reads the
events file.

Migration note: earlier versions loaded `./events.toml` implicitly when it
existed. That probe is gone. Place `events_file = "./events.toml"` in
`media-catalog.toml` (or pass `--events`) to keep loading it; without a
configured path, builds publish a catalog with no events.

```toml
[[events]]
slug = "alps-2024"
title = "Alps trip 2024"
start = "2024-05-28 09:00"
end = "2024-06-05 18:30"
description = "Hiking around Chamonix."

[[events.sections]]
slug = "chamonix"
title = "Chamonix"
at = "2024-05-28 09:00"

[[events.sections]]
slug = "zermatt"
title = "Zermatt"
at = "2024-06-01 08:00"
description = "The Matterhorn leg."
```

- `slug`: stable URL segment using lowercase ASCII letters, digits, and single
  hyphens; unique across the file. `/events/{slug}/` is the event page.
- `title`: required non-empty display name.
- `start`, `end`: inclusive timestamps in `YYYY-MM-DD HH:MM` form; `start` must
  not be after `end`.
- `description`: optional text displayed on the event page.
- `sections`: optional list of bookmark sections on the event. Each section has
  a `slug` using the same rules as event slugs but unique within the event, a
  non-empty `title`, an `at` bookmark timestamp, and an optional `description`.
  Section slugs are scoped to their parent event and are used only as in-page
  fragment ids, never as routes.

An item belongs to an event when its effective capture timestamp, truncated to
the minute, lies inside the inclusive range. The effective timestamp is the EXIF
or video capture time, falling back to file modification time. Membership follows
the timestamp, not the folder: an item stored under `2024/06` with a May capture
time can appear in a May event while its year/month browsing remains `2024/06`.
Overlapping events are allowed and an item may belong to several.

Sections are authored as bookmarks and published as resolved ranges. A section
starts at its `at`, a non-last section ends one minute before the next section's
`at`, and the last section ends at the event end. Boundaries are inclusive and
truncated to the minute like event membership, so an item exactly at the next
bookmark belongs to the later section. Sections are published in chronological
order.

The generator validates slugs, titles, timestamps, and ranges before scanning. A
missing configured events path (from `events_file` or `--events`), duplicate
event or section slug, duplicate section timestamp, malformed timestamp, section
outside its event, or reversed range fails the run before any work starts. A
catalog without an events file publishes no event fields and remains
byte-identical to a catalog built before event support. Events without sections
also remain byte-identical.

Event pages group items by authored section in chronological order when sections
are configured; otherwise they group items by folder month, newest month and
newest item first. Month headings link to the normal `/YYYY/MM/` pages. A
sectioned page includes in-page navigation to each non-empty section, and
sections whose ranges match no items are omitted from both the page and its
navigation. Items outside every published section appear in an "Other" bucket
after the authored sections; the bucket is omitted when empty. The run summary
reports each configured event and its matched item count; an event with more
than 500 items produces a warning because a page that large is a signal to split
the event or add section bookmarks.

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
data/
  catalog/index.json
  catalog/YYYY.json
  thumbs/...
```

`catalog/index.json` is the commit point. The generator writes every referenced
year shard first, replaces the index last, and then removes stale shards. An
interrupted publication therefore leaves the previous complete catalog in place.

The server keeps an immutable in-memory snapshot loaded from the index and its
shards. It checks the index metadata on each catalog request and reloads when it
changes. A reload that cannot read every referenced shard, or that contains
duplicate or undeclared event slugs, event counts that disagree with shard
membership, or inconsistent published sections (duplicate slugs, unparseable or
out-of-range timestamps, unordered or overlapping ranges), is discarded, so the
previous snapshot keeps serving and visitors never see a partial catalog.

Catalog pages are rendered per request with Askama:

- root years index at `/`
- months for a year at `/{year}/`
- lazy-loaded item grid at `/{year}/{month}/`
- event list at `/events/` and one event page per event at `/events/{slug}/`,
  grouped by authored sections when configured or by folder month otherwise

Month grids render every item with:

- `loading="lazy"` and `decoding="async"` thumbnails
- a single-line caption with `YY-MM-DD HH:MM` and the camera model when known
- filename alternative text on every thumbnail
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

- The server requires the media root and starts without a published catalog.
- `/`, `/search.html`, and `/assets/*` remain available before publication.
- `/` shows a bootstrap page stating that the catalog is not available yet.
- `/{year}/` and `/{year}/{month}/` return `503` before publication and `404` for
  unknown years or months.
- `/events/` and `/events/{slug}/` return `503` before publication; an event slug
  absent from the catalog returns `404`.
- `/catalog/*` returns `404` until JSON exists.
- `/healthz` reports catalog readiness separately from media readiness.

When the generator publishes catalog JSON, pages and search data become available
without restarting the server.

## JSON catalog and search

`catalog/index.json` lists years, counts, months, and shard paths. When events are
configured it also includes event metadata ordered by newest start and then slug:

```json
{
  "schema_version": 2,
  "total_items": 2,
  "years": [
    {
      "year": 2024,
      "count": 2,
      "months": [{ "month": 5, "count": 2 }],
      "shard": "catalog/2024.json"
    }
  ],
  "events": [
    {
      "slug": "alps-2024",
      "title": "Alps trip 2024",
      "start": "2024-05-28 09:00",
      "end": "2024-06-05 18:30",
      "description": "Hiking around Chamonix.",
      "sections": [
        {
          "slug": "chamonix",
          "title": "Chamonix",
          "start": "2024-05-28 09:00",
          "end": "2024-06-01 07:59"
        },
        {
          "slug": "zermatt",
          "title": "Zermatt",
          "start": "2024-06-01 08:00",
          "end": "2024-06-05 18:30",
          "description": "The Matterhorn leg."
        }
      ],
      "count": 2
    }
  ]
}
```

Each `catalog/YYYY.json` shard contains ordered item records:

```json
{
  "schema_version": 2,
  "year": 2024,
  "items": [
    {
      "path": "2024/05/a.jpg",
      "month": 5,
      "kind": "image",
      "captured_at": "2024-05-01 10:11:12",
      "model": "iPhone 15 Pro",
      "width": 4032,
      "height": 3024,
      "duration_ms": null,
      "thumb_url": "/thumbs/2024/05/a.jpg.webp?v=...",
      "media_url": "/media/2024/05/a.jpg",
      "thumb_available": true,
      "events": ["alps-2024"]
    }
  ]
}
```

Event fields are additive: `events` is omitted from the index and from every item
when no events are configured, and an event's `sections` array is omitted when it
has no sections. Older catalogs without the fields load as having no events or
sections. Item event slugs are sorted independently of the order events appear
in the events file. The camera model is also optional per item and omitted when
unknown. Sections publish
resolved `start`/`end` ranges rather than bookmark timestamps, so the server does
not depend on the generator's bookmark derivation rule.

Search (`search.html` + `assets/search.js`, both embedded in the server binary)
loads the index and all year shards in parallel, then filters by text, type
(`image`/`video`), year, and month. Filters are reflected in the URL query
(`q`, `kind`, `year`, `month`) for shareable links and restored on page load.
Without JavaScript, the page explains the limitation and links to the browsable
year hierarchy. If catalog JSON cannot be loaded yet, the search page shows a
clear catalog-unavailable message.

## In-page media viewer

`assets/viewer.js` (embedded in the server binary alongside `search.js`) turns
the tiles on a month, event, or search page into an in-page album. A plain click
or keyboard activation opens the item in a modal dialog instead of navigating
away; modifier clicks, middle clicks, and new-tab gestures keep the original tile
link behavior.

- **Album scope:** tiles are collected from the live page in document order, so a
  month page covers that month, a sectioned event page covers every authored
  section and the Other bucket, and a search page covers the current filtered
  results. Navigation stops at the first and last item.
- **Navigation:** use the Previous/Next controls, the Left/Right arrow keys, or a
  horizontal swipe while an image is at fit scale. Arrow keys keep navigating
  while an image is zoomed.
- **Images:** the thumbnail is shown while the original loads, each newly opened
  item starts at fit scale, and pinch, wheel, double-tap/double-click, and
  `+`/`-`/`0` zoom with panning clamped to the viewport. Zoom resets when
  moving to another item, and adjacent images are preloaded after idle unless
  the browser reports a data-saver preference.
- **Videos:** videos play inline with native controls, the thumbnail as poster,
  `playsinline`, and metadata-only preload. They never autoplay, stop and rewind
  when you navigate away or close the viewer, and fall back to the caption and
  original link when playback is unsupported. Arrow keys navigate unless the
  video player itself owns focus, in which case the native player seeks.
- **Deep links:** the open item is stored as `?item=<relative path>` on the
  album URL. Opening, navigating, and closing use `history.replaceState`, so
  browser Back does not step through images. Search filter parameters are
  preserved alongside `item`; a stale item path opens nothing and leaves the
  grid usable.
- **Accessibility:** the viewer is a modal dialog with labelled controls, focus
  containment, an announced current position, and reduced-motion styles.
- **Fallback:** tile anchors stay regular links to `/media/`. If the viewer
  script is unavailable or fails to initialize, tiles continue to open the
  original file directly.

Bearer-token deployments must rely on browser-compatible credentials for
`/media/` subresources; the viewer does not add header-based fetching. See
`docs/server.md` for asset routes and caching behavior.

Browser verification checklist for the video and focus paths:

- Open an image album, click a tile, and confirm the viewer opens in place.
- Navigate with the buttons, arrow keys, and a touch swipe; confirm the first and
  last items disable the matching direction.
- Play a video, navigate away, and confirm audio stops; reopen and confirm it
  starts paused.
- Focus the video player and press Left/Right to confirm native seeking; move
  focus off the player and confirm the arrows navigate again.
- Close with the control, backdrop, and Escape and confirm focus returns to the
  originating tile.
- Reload a page with `?item=` and confirm the same item reopens; confirm Back
  does not step through previously viewed images.

JSON and rendered pages contain no build timestamps, so two runs over an
unchanged archive produce byte-identical output.

## Incremental builds, resume, and cleanup

The fingerprint is `size:mtime` (nanosecond modification time), stored in SQLite
with cached probe metadata and thumbnail status.

Schema version 2 adds the camera model column. Opening a version-1 state database
migrates it in place and clears cached probe metadata once so unchanged items are
probed again without regenerating thumbnails. Published catalog JSON is also
schema version 2, so rebuild the catalog before the updated server accepts it.

On each run:

1. Scan the media tree and diff fingerprints.
2. Probe only new/changed items, plus unchanged rows missing cached metadata.
3. Thumbnail only new/changed items, retries, missing files, and `--force` work.
4. Publish JSON shards, then `catalog/index.json`, then remove stale shards.
5. Remove state rows for deleted items (unless blocked by the mount guard).
6. Optionally prune orphan thumbnails under `<data-root>/thumbs/`.

Unchanged sources are not reopened for probing or thumbnailing. An interrupted
build leaves completed thumbnails recorded and in place; rerunning resumes the
unfinished work and retries failed items. Deleted items disappear from JSON and
therefore from server-rendered pages immediately.

### Prune safety and mount drops

Pruning is opt-in with `--prune` and is confined to the thumbnail tree in the
data root. It removes files that do not belong to any current catalog item and
cleans up empty directories; no file outside the thumbnail tree is touched.

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
  --data-root "$HOME/.media-catalog/data" \
  --media-root /Volumes/media/photos \
  --bind 127.0.0.1:8080
```

It streams `/media/` with byte ranges and conditional requests, serves catalog
pages from JSON, and exposes `/healthz`. It binds to loopback by default, reads
the access token from `MEDIA_CATALOG_TOKEN`, and never writes to either root.

Keep the data root and the state root on local disk; SQLite WAL is not supported
on network filesystems. The media root can stay on the NAS. Systemd and launchd
examples live in [`docs/server.md`](docs/server.md).

A TLS-terminating reverse proxy may sit in front of the server; it must pass
`Authorization`, `Range`, and `If-Range` and avoid buffering `/media/`. The
server itself speaks plain HTTP.

### Upgrading from the catalog-root layout

Older deployments used `--catalog-root` with a `public/` wrapper. The binaries no
longer accept `--catalog-root`, and neither binary falls back to a legacy
`public/` subtree. Move the published tree and the build state into their own
roots:

```sh
mv catalog/public data
mkdir -p state
mv catalog/state.sqlite catalog/build.lock state/
```

Then update commands and configuration:

```sh
media-catalog        --media-root ./media --data-root ./data --state-root ./state
media-catalog-server --media-root ./media --data-root ./data
```

In TOML, replace `catalog_root = "./catalog"` with `data_root = "./data"` and
`state_root = "./state"`.

Older builds may also have left obsolete static files inside the moved tree:

```text
data/index.html
data/YYYY/index.html
data/YYYY/MM/index.html
data/search.html
data/assets/
```

The server ignores them and renders pages and assets itself. Remove them when
convenient:

```sh
find data -type f -name index.html -delete
rm -f data/search.html
rm -rf data/assets
```

Do not remove `data/catalog/` or `data/thumbs/`; those are the runtime data.
Rollback requires the previous binary and moving the directories back. The JSON
and thumbnail contents themselves are unchanged.

### Initial build

The first complete build reads the entire archive and can take hours for a large
NAS collection. Use a modest `--jobs` value (for example 4) to bound memory on
large panoramas and avoid hammering the NAS. If the process is interrupted, rerun
the same command; completed thumbnails are skipped. Later incremental runs
process only new or changed files and are much faster.

## Run summary and exit status

The summary reports scanned counts, new/changed/unchanged items, probes,
thumbnails generated/skipped/failed/unsupported, JSON files written, deleted
rows, pruned files, per-item errors, elapsed time, and each configured event with
its matched item count. Events that match more than 500 items add a warning.
Exit status is nonzero for configuration errors, any per-item failure, or a
blocked mount-drop guard.
