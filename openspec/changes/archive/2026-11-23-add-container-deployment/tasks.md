# Tasks

## 1. Server image

- [x] 1.1 Add a root `.dockerignore` excluding `.git`, `target/`, `media/`, `data/`, `state/`, and local deployment artifacts; verify a build from the repository root sends none of those directories by inspecting the builder's file-transfer output.
- [x] 1.2 Add a multi-stage `Dockerfile` using a pinned Rust build image and `debian:bookworm-slim` runtime with `curl` for health checks; copy `src/`, `templates/`, and `assets/`, run `cargo build --locked --release --bin media-catalog-server`, and copy only that binary into the runtime stage; verify `docker build -t media-catalog-server:test .` succeeds and the binary answers `--help`.
- [x] 1.3 Add the runtime entrypoint that defaults to `media-catalog-server --config /etc/media-catalog/media-catalog.toml` and forwards supplied arguments, plus a token-aware healthcheck script; verify default startup, argument override, and healthcheck behavior with and without `MEDIA_CATALOG_TOKEN`.
- [x] 1.4 Verify the runtime image runs as a numeric non-root user, contains no `media-catalog` importer binary, and contains no `ffmpeg` or `ffprobe`; run `id -u`, `command -v media-catalog`, `command -v ffmpeg`, and start the server against a temporary catalog.
- [x] 1.5 Document the server image build, entrypoint, non-root user, read-only runtime, healthcheck, and deliberate absence of importer tools in `docs/deploy/docker.md`; verify every documented command runs as written against the built image.

## 2. Compose example

- [x] 2.1 Add `compose.yaml` with exactly one `server` service built from the image, publishing port 8080, mounting the config file, `/media`, and `/data` read-only, and mounting no state directory; apply `no-new-privileges`, `cap_drop: ALL`, a read-only root filesystem, and a `/tmp` tmpfs; verify `docker compose config` accepts the file and lists only the server service.
- [x] 2.2 Add an example `config/media-catalog.toml` and `.env.example` with container paths (`/media`, `/data`), `bind = "0.0.0.0:8080"`, `PUID`/`PGID`, and an optional empty token; verify `docker compose config` resolves every mount and environment value.
- [x] 2.3 Document the three-mount contract, host-directory read permissions, external import prerequisite, startup-before-catalog behavior, token/reverse-proxy behavior, and rollback in `docs/deploy/docker.md`; verify the documented setup serves a catalog produced by the existing host importer.
- [x] 2.4 Add a Docker deployment link to the README deployment section; verify the link resolves and clearly states that the container does not include the importer, `ffmpeg`, or a scheduler.

## 3. Integration verification

- [x] 3.1 Build the server image and start it before any catalog exists; verify `/` serves the bootstrap response and `/healthz` reports catalog unavailability without crashing.
- [x] 3.2 Run the existing host importer against a small temporary media archive to produce `data/catalog/index.json`, year shards, thumbnails, and `state/state.sqlite`; start the container server with `data/` and the media root read-only and verify `/healthz` returns `200` and rendered pages load.
- [x] 3.3 Publish a second catalog with the external importer while the server container is running; verify the server serves the updated catalog without a restart.
- [x] 3.4 Verify the server container has no `/state` mount and cannot write under `/data` or `/media`; verify catalog reads and media streaming still work.
- [x] 3.5 Verify token behavior: with `MEDIA_CATALOG_TOKEN` set, the healthcheck succeeds and an unauthenticated request returns `401`; with the token empty, the same request succeeds.
- [x] 3.6 Verify the built image contains only the server binary and no importer or media tools; inspect the runtime layer and run the absence checks from task 1.4.
- [x] 3.7 Run `openspec validate add-container-deployment --strict` and verify the change passes after the artifacts and documentation are complete.

## Workflow follow-up

- Archive the change after implementation and review are complete.
- Adapt the existing host import job or create a separate importer deployment outside this change.
- Verify the archived container-deployment spec and remove the temporary build image.
