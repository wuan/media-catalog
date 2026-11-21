# Proposal

## Why

The generated catalog assumes a web server can expose the NAS originals at `/media/` via a filesystem alias. That may not be possible when the web host cannot mount the SMB share. The catalog then has metadata and thumbnails but no working original-media links. A small Rust runtime service can serve the existing static catalog and stream originals from a host that does have the share mounted.

## What Changes

- Add a read-only HTTP server mode that serves the generated `public/` tree (`HTML`, `JSON`, assets, thumbnails) at `/`.
- Stream original media from the configured media root at `/media/`, including `HEAD`, byte-range requests, ETag revalidation, and correct content types.
- Reject path traversal and symlink escape; never list directories or write to the media root.
- Add operational behavior: configurable bind address, optional access token/basic auth, request logging, mount-health checks, and graceful shutdown on `SIGINT`/`SIGTERM`.
- Keep metadata serving static and database-free at runtime. No dynamic search API or SQLite access is introduced.
- Package the server separately from the generator pipeline so build dependencies remain unchanged.

## Capabilities

### New Capabilities

- `media-server`: serving the generated catalog and streaming read-only original media over HTTP from a host with access to the media root.

### Modified Capabilities

_None._

## Impact

- New runtime component and dependencies (HTTP server, async runtime, static-file and streaming utilities).
- New deployment topology: the service must run on a host that can read the media root, or a media gateway must be deployed there and proxied.
- Existing generator output and URL conventions stay unchanged; `/media/` and `/thumbs/` remain same-origin and root-relative.
- Direct streaming does not imply browser playback: formats such as HEVC or MKV may still require download or a separate transcoding capability.
- Reverses the original design non-goal of no application server at runtime.
