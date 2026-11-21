# Proposal

## Why

The family media archive currently holds ~75,000 photos and videos (523 GB) laid out as `YYYY/MM`, with ~6,000 new files per year. Browsing it means walking raw NAS directories, which is slow and gives no thumbnails, no month-over-month overview, and no way to search without opening originals. A prebuilt static catalog keeps delivery fast on the LAN while leaving the originals untouched.

## What Changes

- Add a new Rust CLI that reads a read-only media root organized as `YYYY/MM` and writes a separate, deployable static web tree.
- Scan and classify photos and videos, extract capture metadata, and keep folder grouping authoritative.
- Generate one 512x512 cover-crop WebP thumbnail per item. Photos decode via the `image` crate; videos use `ffmpeg` to probe and grab a poster frame. HEIC is deferred behind a decoder boundary.
- Render static HTML: a years index, per-year month indexes, and per-month item grids with lazy-loaded thumbnails.
- Emit sharded JSON (`catalog/index.json`, `catalog/YYYY.json`) plus a static `search.html` that filters by text, type, year, and month in the browser.
- Keep build state in SQLite so reruns are incremental: changed files regenerate thumbnails, unchanged files are skipped, interrupted runs resume, and stale thumbs can be pruned.
- Use same-origin, root-relative URLs (`/thumbs/...`, `/media/...`) so the web tree and the NAS-mounted originals can be served by one web server.

## Capabilities

### New Capabilities

- `media-inventory`: scanning the `YYYY/MM` media tree, classifying files, extracting capture metadata, and producing the ordered catalog model.
- `thumbnail-generation`: turning photos and video frames into cacheable 512x512 thumbnails, including fingerprint versioning and deferred HEIC support.
- `static-catalog`: rendering the static HTML hierarchy, JSON shards, and client-side filter page with same-origin URL conventions.
- `incremental-builds`: SQLite-backed state, change detection, resumable builds, locking, and orphan cleanup.

### Modified Capabilities

_None._

## Impact

- New Rust project at the repository root (Cargo crate, binary plus library modules, templates, assets, tests).
- New runtime dependency on `ffmpeg`/`ffprobe` for video probing and poster extraction.
- Assumes a web server (e.g. nginx) serving `public/` at `/` with the media root mounted at `/media/`; no database or application server is introduced.
- No existing code or APIs are affected.
