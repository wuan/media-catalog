# Design

## Context

See `proposal.md` for motivation. The generator already emits a deterministic `public/` tree with same-origin `/thumbs/` and `/media/` URLs. The current deployment assumes a web server can alias the NAS media root; this design removes that assumption by introducing a read-only HTTP service.

Constraints carried over from the existing system:

- `public/` and `state.sqlite` should live on local disk so SQLite WAL is reliable.
- The media root is read-only and may be an SMB mount.
- The server must not require SQLite or any generator state at runtime.
- URLs written into HTML/JSON must continue to resolve without changing the generator output.

## Goals / Non-Goals

**Goals:**

- Serve the generated catalog and stream originals from one small read-only service.
- Correct `HEAD`, single-range, conditional, and content-type behavior for media.
- Safe path handling and no directory traversal or listings.
- Predictable caching: immutable thumbnails, revalidated HTML/JSON and originals.
- Clear behavior when the media mount disappears.
- Graceful shutdown and simple operational controls.

**Non-Goals:**

- Video transcoding, remuxing, HLS, or on-demand thumbnails.
- Uploads, tagging, authentication/authorization beyond a single access credential.
- Dynamic search or metadata APIs backed by SQLite.
- Speaking the SMB protocol inside Rust.
- Replacing the generator or changing its output.

## Decisions

### Deploy the service on a host that can read the media root

Primary topology: one service process runs where the media root is mounted (for example the Mac/build host) and serves both `public/` from local disk and `/media/` from the mount.

```
Browser
   |
   v
+--------------------------------------------------+
| media-catalog-server                             |
|  /                  -> public/                   |
|  /assets, /catalog  -> public/                   |
|  /thumbs/*          -> public/thumbs/*           |
|  /media/*           -> media_root/*              |
+--------------------------+-----------------------+
                           | read-only
                           v
                    SMB media mount
```

If the frontend host cannot mount SMB, deploy this service as a media gateway on a host that can, and reverse-proxy `/media/` to it from the frontend. Alternative considered: implement an SMB client in Rust. Rejected because SMB crates are immature and would duplicate authentication, locking, caching, and reconnect behavior that the OS mount already provides.

### Separate server binary, not a generator subcommand

Add a second binary target rather than folding serving into the existing build CLI. The generator remains a batch pipeline; the server has different dependencies, lifetime, and failure modes. This keeps `media-catalog` invocation unchanged and lets the server run under `launchd`/`systemd` independently.

Alternative considered: `media-catalog serve` subcommand. Convenient, but couples a long-running daemon to the build tool and makes a future workspace split harder. Alternative considered: separate workspace crate. Cleaner dependency isolation, but a larger repository restructure than needed for v1; the server can be extracted later if compile times or dependency weight become a problem.

### Axum, Tokio, and tower-http

Use `axum` for routing, `tokio` for async I/O, and `tower-http` for static-file serving and middleware. The `/media/` route gets a custom handler because video seeking requires explicit range handling; generic static-file services may not guarantee the exact `HEAD`/`206`/`416` behavior this contract needs.

Alternative considered: `actix-web` (mature, fast, but a larger runtime/framework surface) and raw `hyper` (minimal, but reimplements routing and static-file concerns).

### Route shape

```text
GET|HEAD /                    -> public/index.html
GET|HEAD /{year}/             -> public/{year}/index.html
GET|HEAD /{year}/{month}/     -> public/{year}/{month}/index.html
GET|HEAD /assets/*            -> public/assets/*
GET|HEAD /catalog/*           -> public/catalog/*.json
GET|HEAD /thumbs/*            -> public/thumbs/*
GET|HEAD /media/*             -> stream from media_root
GET      /healthz             -> liveness/readiness
```

There is no fallback to a directory index. Unknown paths return `404`; `/media/` directory paths return `404`. The server never serves `state.sqlite`, `build.lock`, or anything outside `public/` and `media_root`.

### Range and streaming

The `/media/` handler:

- opens the file, reads metadata, and computes `size`, `Last-Modified`, and a strong `ETag` from size + mtime;
- handles `HEAD` by returning headers with no body;
- parses a single `Range: bytes=start-end`, supports open-ended ranges, and returns `206` with `Content-Range` and exact `Content-Length`;
- returns `416` with `Content-Range: bytes */<size>` when unsatisfiable;
- streams through `tokio::fs::File` + a bounded reader stream, with backpressure from the HTTP body;
- limits concurrent media streams with a semaphore to bound SMB load.

Multi-range requests are not supported in v1; they get `416` rather than a multipart response.

Alternative considered: `sendfile`/zero-copy. Not initial scope; SMB read throughput is likely the bottleneck, and file streaming is portable across macOS and Linux. Measure before optimizing.

### Path safety

The handler percent-decodes once, rejects empty segments, `.`, `..`, backslash, NUL, and control characters, then joins the relative path under `media_root`. Before opening, it canonicalizes the resolved path and the root and verifies the resolved path is still inside the root. This blocks encoded traversal and symlink escape. The public tree uses the same rule.

Alternative considered: lexical checks only. Faster, but symlinks inside the media tree could escape the root; canonicalization is worth the metadata cost until measurements show it matters.

### Caching policy

- Versioned thumbnails (`?v=...`): `Cache-Control: public, max-age=2592000, immutable`.
- HTML and JSON: revalidation (`Cache-Control: no-cache`) with `ETag`/`Last-Modified`.
- Originals: `ETag` and `Last-Modified`, `Cache-Control: private, no-cache` (or private with a short max-age), honoring `If-None-Match` and `If-Modified-Since`.
- `Content-Type` from a whitelisted extension map plus `X-Content-Type-Options: nosniff`; `Content-Disposition: inline` for browser-friendly types.
- Responses for missing files or an unavailable mount are not cached.

### Mount health

At startup, verify that `media_root` exists and is a directory. On `/media/` requests, distinguish a missing mount from a missing file: if the root is absent or not a directory, return `503` with a short explanatory body. Do not cache negative results. A lightweight `/healthz` can report readiness based on the same check.

This complements the generator's mount-drop guard: the build refuses destructive cleanup, while the server refuses to pretend the archive is empty.

### Binding and access control

Default bind is loopback (`127.0.0.1`) so an accidental start is not exposed to the LAN. Operators opt into a LAN/Tailscale address explicitly. Access control is optional and intentionally simple: a bearer token or HTTP Basic credential read from an environment variable rather than the command line, checked before serving media. This is not a multi-user authorization system.

Alternative considered: no auth at all. Acceptable on a trusted LAN, but a token costs little and protects against accidental exposure. TLS is left to a reverse proxy or a later change.

### Graceful shutdown

Use the HTTP server's graceful-shutdown hook. First `SIGINT`/`SIGTERM` stops accepting new connections and lets active responses drain; a second signal forces exit. The listener is closed on exit so a restart does not hit "address already in use".

This deliberately gives the server better shutdown behavior than the current build pipeline, which relies on resume semantics after interruption.

### Testing strategy

- Integration tests with temporary public and media trees: static routes, Unicode paths, spaces, `HEAD`, single range, unsatisfiable range, `304`, missing files, directory requests.
- Security tests: `..`, encoded traversal, backslash, NUL, symlink escape.
- Mount tests: missing media root returns `503`; restored root serves normally.
- Auth tests: missing/invalid/valid credential.
- Shutdown test: interrupt while a response is in flight, confirm drain and port release.
- Streaming test: a file larger than the read buffer returns the exact byte sequence.
- Unit tests for range parsing and content-type selection.

## Risks / Trade-offs

- **[SMB mount disappears while serving]** -> Return `503`, do not cache, keep the process alive; optionally log readiness transitions.
- **[SMB throughput or latency]** -> Bound concurrent streams, use buffered sequential reads, measure before adding zero-copy or caching layers.
- **[Browser codec support]** -> Direct streaming plays browser-native formats only; HEVC/MKV may download instead. Transcoding is explicitly out of scope.
- **[Path traversal or symlink escape]** -> Decode once, reject unsafe segments, canonicalize, and re-check containment before opening.
- **[Accidental public exposure]** -> Loopback default, explicit bind, optional token, reverse-proxy TLS; document firewall expectations.
- **[Long-running process needs supervision]** -> Provide `launchd`/`systemd` examples and a health endpoint; process restart is safe because the service is read-only.
- **[Static catalog changes during serving]** -> Generator already writes atomically, so readers see old or new complete files, not partial output.
- **[Dependency weight in the crate]** -> Separate binary target is a deliberate compromise; extract a workspace member if build times or audit scope demand it.

## Migration Plan

1. Keep `public/` and `state.sqlite` on local disk; fix the catalog root away from SMB before introducing the service.
2. Build or deploy the server binary and run it on the host with the media mount.
3. Start on loopback, verify static pages, thumbnails, media `GET`/`HEAD`, range seeking, and `304` behavior.
4. Move to the LAN/Tailscale bind address and enable the access token if exposure is broader than expected.
5. If nginx remains, leave `/` and `/thumbs/` on nginx and reverse-proxy `/media/` to the Rust gateway; otherwise let the service serve everything.
6. Rollback: stop the service and restore the previous nginx alias or deployment; the generator output is unchanged.

## Open Questions

- Whether the frontend host can mount the media root directly or needs a separate gateway deployment.
- Whether browser playback for HEVC/MKV is required; if so, transcoding becomes its own change.
- TLS termination: reverse proxy versus embedded rustls.
- Whether to extract the server into a workspace crate once dependencies stabilize.
