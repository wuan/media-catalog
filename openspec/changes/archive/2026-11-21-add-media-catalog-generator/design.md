# Design

## Context

See `proposal.md` for motivation. The repository is greenfield: only a README and OpenSpec scaffolding exist. The archive lives on a NAS as `YYYY/MM`, currently ~75,000 items / 523 GB, growing ~6,000 files per year. The web tree is served by one web server at `/`, while the NAS originals are mounted at `/media/`. Builds are run manually after updates, not by a watcher.

The design centers on three constraints:

- The static web tree must be cheap to deploy and fast to browse on the LAN.
- Originals must never be modified or copied.
- A rerun after adding a few hundred files must not reprocess 75,000 files.

## Goals / Non-Goals

**Goals:**

- One Rust binary that scans, probes, thumbnails, renders, and indexes in a single run.
- Incremental reruns driven by a persistent SQLite state.
- A deployable `public/` tree containing only HTML, thumbnails, CSS, JS, and JSON.
- Deterministic output so unchanged catalogs produce identical files.
- Safe interruption and resume.

**Non-Goals:**

- HEIC decoding in v1 (decoder boundary only).
- Multiple thumbnail sizes, smart cropping, or face-aware crops.
- Per-item HTML pages, lightbox playback, or video transcoding.
- Tags, favourites, GPS maps, authentication, upload UI.
- A daemon or filesystem watcher.
- Database or application server at runtime.

## Decisions

### Rust CLI with a library boundary

Use a single Cargo package with `src/lib.rs` for the pipeline and a thin `src/main.rs` for CLI wiring. Modules:

```
config.rs       clap + optional TOML
scan.rs         walkdir, extension classification, YYYY/MM parsing
fingerprint.rs  size + mtime fingerprint and thumbnail version
probe.rs        image dimensions/EXIF, ffprobe for video
thumbs.rs       decoder strategy, crop/resize/encode, atomic writes
state.rs        SQLite schema, upserts, run tracking, lock
model.rs        in-memory catalog model (years/months/items)
render.rs       askama templates, HTML pages, assets
json.rs         sharded JSON output
paths.rs        URL encoding, relative-path mapping
report.rs       progress and run summary
```

Keeping the pipeline in a library makes integration tests possible without spawning the binary. `askama` gives compile-time templates and automatic HTML escaping; assets are embedded with `include_str!`.

### Decoder boundary instead of HEIC-specific handling

Photos decode through the `image` crate (JPEG, PNG, WebP). Videos use `ffmpeg`/`ffprobe` via `std::process::Command` to probe and extract one frame, then the frame enters the same crop/encode path as photos. This keeps photo and video thumbnails visually consistent and limits ffmpeg to "probe + frame grab."

A `MediaDecoder` boundary separates probing and thumbnailing. v1 registers the raster and video decoders; HEIC can be added later with `libheif`/`libvips` without changing the pipeline, state schema, or renderer.

Alternative considered: libvips for everything. It is faster and handles HEIC, but adds a native system dependency for all photos when the archive has very little HEIC. Deferred until HEIC support is actually needed.

### Pipeline order

```
config -> lock -> open state -> scan -> fingerprint diff
       -> probe changed -> thumbnail changed -> update state per success
       -> build model -> render all pages + JSON -> optional prune -> report
```

HTML and JSON are regenerated on every run. They are cheap relative to decoding and make deletion, rename, and ordering changes automatic. Thumbnails are the expensive part and are incremental.

Probing is also skipped for unchanged items: width, height, duration, and capture timestamp are cached in SQLite alongside the fingerprint.

### SQLite schema

State lives outside `public/`, in `state.sqlite` directly under the catalog root. The catalog root owns both the deployable `public/` tree and build metadata, so no configuration path needs to climb out of `public/`.

```sql
CREATE TABLE items (
  path          TEXT PRIMARY KEY,   -- relative to media root
  year          INTEGER NOT NULL,
  month         INTEGER NOT NULL,
  kind          TEXT NOT NULL,      -- image | video
  size          INTEGER NOT NULL,
  mtime         INTEGER NOT NULL,
  fingerprint   TEXT NOT NULL,
  width         INTEGER,
  height        INTEGER,
  duration_ms   INTEGER,
  captured_at   TEXT,               -- naive local "YYYY-MM-DD HH:MM:SS"
  thumb_path    TEXT,
  thumb_version TEXT,
  thumb_status  TEXT NOT NULL,      -- pending | ok | failed | unsupported
  error         TEXT,
  last_seen_run INTEGER NOT NULL
);

CREATE INDEX items_month ON items(year, month);

CREATE TABLE runs (
  id INTEGER PRIMARY KEY, started_at TEXT, finished_at TEXT,
  status TEXT, item_count INTEGER, error_count INTEGER
);

CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT);
```

Scan upserts rows and stamps `last_seen_run`. Rows not seen in the run are deleted after rendering, which removes deleted items from pages and JSON. Fingerprint is `size:mtime`, and the thumbnail version is an URL-safe encoding of the same pair. `mtime` uses nanoseconds when available.

Alternative considered: JSON manifest. Simpler, but at 75k rows it means rewriting a large file each run and risking truncated state. SQLite gives transactional per-item updates.

### Thumbnail naming and cache versioning

Thumbnails mirror the source path under `public/thumbs/`, with `.webp` appended:

```
/media/2024/05/event/IMG_0001.jpg
/thumbs/2024/05/event/IMG_0001.jpg.webp?v=<version>
```

Appending rather than replacing the extension avoids collisions between same-stem files and keeps names debuggable. Mirroring subdirectories prevents collisions in nested month folders. The version query changes only when size or mtime changes, giving stable immutable-ish paths with cheap cache busting.

Each thumbnail is written to a temporary file in the destination directory and renamed into place. Failed thumbnails are recorded with status `failed`; the next run retries them even when the fingerprint is unchanged.

### Crop and orientation

Orientation is applied before cropping. Photos use EXIF orientation and videos rely on ffmpeg's autorotation. The crop is a center square computed from the upright dimensions: scale so the shorter side covers 512, crop the center, then encode WebP at quality ~80. No metadata is carried into the output.

Video poster time is `min(1 second, 10% of duration)` with a first-frame fallback for very short or unseekable videos.

### Deterministic ordering and render model

The model is built from SQLite rows plus the current scan. Years and months render newest-first. Within a month, items sort by capture timestamp, then relative path. Missing capture timestamps fall back to mtime, stored as a naive local time string; sorting is lexicographic. This avoids timezone ambiguity from EXIF timestamps that carry no zone.

Every page is rendered from the same model, so HTML and JSON cannot disagree. Build timestamps are not embedded in catalog pages or JSON, preserving byte-identical reruns.

### HTML and JSON shape

```
public/
  index.html
  YYYY/index.html
  YYYY/MM/index.html
  search.html
  assets/site.css
  assets/search.js
  catalog/index.json
  catalog/YYYY.json
  thumbs/...
```

Month pages contain the month's item grid with `loading="lazy"` and `decoding="async"`; at ~500 items/month this is a modest HTML document, and thumbnails load as needed. Each tile shows the thumbnail, capture date, a video marker and duration for videos, and links the original under `/media/`. Missing thumbnails render a neutral placeholder tile rather than breaking the page.

The search page uses vanilla JS: fetch `catalog/index.json`, fetch year shards in parallel, filter by text/type/year/month, and render tiles. Filters are reflected in the URL query for shareable links. Without JS, the page links to the browse hierarchy.

URLs are built from configurable prefixes (`/media`, `/thumbs`) and percent-encoded per path segment. `askama` escapes text; filenames are never interpolated into raw HTML attributes without encoding.

### Locking, atomic writes, and pruning

A lock file beside the SQLite database is held for the whole run. A second run exits with a clear message. Pages, JSON, and thumbnails are written via temp-file + rename. Pruning is opt-in: walk `public/thumbs/`, compare against the expected thumb path set, delete files outside it, and remove empty directories. The prune walk is constrained to the thumbnail tree.

### Configuration

CLI flags override an optional TOML file:

```
--media-root <path>          default: required
--catalog-root <path>        default: required; contains public/, state.sqlite, and build.lock
--media-url-prefix <path>    default: /media
--thumb-url-prefix <path>    default: /thumbs
--jobs <n>                   default: min(4, cores)
--force                      regenerate thumbnails
--prune                      delete orphan thumbnails
--dry-run                    scan and report without writing
--quiet
```

`--jobs` defaults low to bound memory for large panoramas and avoid hammering a NAS.

### Error handling and reporting

Per-item errors never abort the whole build. Failures are recorded in state and listed in the summary. Exit status is nonzero when any item failed or when configuration is invalid. A progress bar is shown on a TTY; `--quiet` suppresses it. The summary reports scanned, new, changed, unchanged, thumbnails generated, skipped, errors, and elapsed time.

### Testing strategy

- Unit tests: fingerprint/version, YYYY/MM parsing, URL segment encoding, crop math, ordering tie-breaks, HTML escaping.
- Integration tests: temporary media tree with generated tiny images, one run, assert pages/JSON/thumbs; second run is idempotent; changed file regenerates one thumb; deleted file disappears and prune removes its thumb; lock blocks a concurrent run.
- Video tests use a small checked-in fixture or are skipped when `ffmpeg` is not installed.
- Golden-file snapshots (`insta`) for representative HTML and JSON keep output stable.

## Risks / Trade-offs

- **[Initial build reads 523 GB]** → It is I/O-bound (~1–2 h at 100 MB/s). Incremental runs are small. Resume and per-item state make interruption safe.
- **[Memory spikes on very large images]** → Bound worker count via `--jobs`; process one item per worker and let failures surface per item.
- **[ffmpeg dependency]** → Document the requirement; probe availability at startup and fail with a clear configuration error before scanning.
- **[HEIC appears]** → Report as unsupported in v1; the decoder boundary allows a later implementation without schema changes.
- **[mtime changes without content changes]** → Occasional unnecessary rethumb, acceptable versus hashing 523 GB. `--force` remains available.
- **[NAS outages or stale mounts]** → Scanning sees fewer files and would treat them as deletions. Mitigate with a mount sanity check and a warning when the item count drops sharply, requiring an explicit override before destructive pruning.
- **[Duplicate/basename collisions]** → Mirroring relative paths under `thumbs/` and appending `.webp` avoids collisions.
- **[Large month pages]** → ~500 items is acceptable with lazy loading; pagination remains a later change if months grow.
- **[Color profiles]** → `image` may drop ICC profiles; colors can shift slightly. Acceptable for thumbnails; revisit if originals are wide-gamut.
- **[Orphan pruning is destructive]** → Opt-in, confined to `public/thumbs/`, and guarded by path checks.
- **[Browser caching]** → Version query invalidates only changed thumbnails; HTML/JSON regenerated each run can use normal revalidation.

## Migration Plan

This is a new project, so there is no data migration. Deployment:

1. Build the release binary.
2. Run the initial full build (expect hours on first run).
3. Deploy `public/` to the web server with a snapshot or rsync.
4. Configure the web server: `/ -> public/`, `/media/ -> NAS mount`, read-only media access.
5. For later updates, run the binary again and rsync the changed tree.

Rollback: keep the previous `public/` snapshot; thumbnails are derived data and can be regenerated from originals.

## Open Questions

- Whether a small thumbnail size should be added later for denser grids; the current URL/naming scheme can accommodate a size segment without changing source paths.
- Exactly how nginx should cache HTML, JSON, and versioned thumbnails. This is deployment configuration, not generator behavior.
