# Tasks

## 1. Server scaffold and configuration

- [x] 1.1 Add the HTTP server dependencies (`axum`, `tokio`, `tower-http`, `mime_guess`, `tokio-util`) and a second binary target; verify `cargo build --bins` succeeds and the existing generator binary still runs.
- [x] 1.2 Implement server configuration for catalog root, media root, bind address, access token, and stream limit; verify invalid or missing roots fail with a clear message before the listener starts.
- [x] 1.3 Add startup validation that `public/` exists, `media_root` is a directory, and the server never exposes `state.sqlite`, `build.lock`, or files outside those roots; verify with an integration test.

## 2. Static catalog serving

- [x] 2.1 Serve `/`, `/{year}/`, and `/{year}/{month}/` from the generated `public/` tree without directory listings; verify HTML status and body for a fixture catalog.
- [x] 2.2 Serve `/assets/*`, `/catalog/*.json`, and `/thumbs/*` with correct content types and `404` for unknown files; verify JSON and thumbnail responses.
- [x] 2.3 Ensure encoded/Unicode and space-containing paths resolve correctly; verify against fixture names such as `Familie Müller #1.jpg`.
- [x] 2.4 Add server tests for static routing, unknown paths, and directory requests; verify `cargo test` covers each case.

## 3. Media streaming

- [x] 3.1 Implement `GET /media/*` streaming from the media root with a whitelisted content-type map and `X-Content-Type-Options: nosniff`; verify image and video fixture responses.
- [x] 3.2 Implement `HEAD` and single-range requests (`206`, `Content-Range`, exact length, `416` for unsatisfiable ranges); verify seeking into a file larger than the read buffer.
- [x] 3.3 Bound concurrent media streams with a semaphore and stream with backpressure; verify a configured limit is enforced under parallel requests.
- [x] 3.4 Add media tests for full response, `HEAD`, valid range, open-ended range, unsatisfiable range, missing file, and byte-exact streaming; verify with `cargo test`.

## 4. Path safety and mount health

- [x] 4.1 Decode paths once and reject empty segments, `.`, `..`, backslash, NUL, and control characters; verify traversal and encoded-traversal tests return `400`/`404`.
- [x] 4.2 Canonicalize resolved paths and verify containment, including symlink escape attempts; verify the server never reads outside the configured root.
- [x] 4.3 Return `503` for `/media/` when the media root is absent or unmounted, and resume serving when it returns; verify with a mount-removal integration test.
- [x] 4.4 Implement `/healthz` reporting readiness based on catalog and media-root availability; verify readiness changes when the media root is removed.

## 5. Caching and conditional requests

- [x] 5.1 Emit strong `ETag` and `Last-Modified` for media and catalog files and honor `If-None-Match`/`If-Modified-Since` with `304`; verify revalidation tests for both.
- [x] 5.2 Set cache policies: immutable for versioned thumbnails, revalidation for HTML/JSON, and private/no-cache for originals; verify headers on fixture responses.
- [x] 5.3 Ensure `404` and `503` responses are not cached; verify cache headers on error responses.

## 6. Binding and access control

- [x] 6.1 Default to a loopback bind address and require an explicit address for LAN exposure; verify the default listener is not reachable externally.
- [x] 6.2 Implement optional bearer-token or Basic authentication sourced from the environment; verify missing and invalid credentials return `401` and valid credentials serve normally.
- [x] 6.3 Document credential handling so secrets are not passed on the command line; verify the documented environment-variable flow works.

## 7. Lifecycle, logging, and shutdown

- [x] 7.1 Add request logging (method, path, status, bytes, duration) without logging credentials; verify log output for a normal and a failed request.
- [x] 7.2 Implement graceful shutdown on `SIGINT`/`SIGTERM`: stop accepting, drain in-flight responses, close the listener, and force exit on a second signal; verify with an in-flight transfer test and a port-reuse check.
- [x] 7.3 Add a startup log with the bind address, catalog root, and media root, plus a clear warning when the media root is unavailable; verify messages in a fixture run.

## 8. Integration, deployment, and documentation

- [x] 8.1 Add an end-to-end integration test that starts the server against temporary public/media trees and checks HTML, JSON, thumbnails, media, range, `304`, traversal rejection, auth, and shutdown.
- [x] 8.2 Document the server deployment topology: local catalog root, media mount on the serving host, and the gateway/reverse-proxy option when the frontend cannot mount SMB; verify the documented commands run against a fixture.
- [x] 8.3 Add `launchd` and `systemd` examples plus firewall/TLS guidance; verify the example unit files parse and the documented bind behavior matches the implementation.
- [x] 8.4 Document that direct streaming does not transcode, and list browser-native versus download-only formats; verify the README matches tested behavior.

## Workflow follow-up

- Archive the change after review and verification pass.
- Verify the archived result and update the deployment documentation if the chosen topology changes.
