# Design

## Context

See `proposal.md` for motivation and `specs/container-deployment/spec.md` for
the behavior contract. The relevant existing constraints are:

- The server needs only two storage roots: `media_root` (originals) and
  `data_root` (`catalog/` + `thumbs/`).
- The server must not open or mount the state root.
- The server binds loopback by default and reads its optional token from
  `MEDIA_CATALOG_TOKEN`.
- Publication order writes catalog shards first and `catalog/index.json` last, so
  a running server reloads a completed catalog without a restart.
- The importer is the only component that needs `ffmpeg`/`ffprobe`, writes build
  state, and acquires `build.lock`.

## Goals / Non-Goals

**Goals:**

- One reproducible image containing only the runtime server.
- A single-service Compose example with read-only media and catalog mounts.
- Clear separation between the server image and the importer.
- Correct permission, token, and readiness guidance.

**Non-Goals:**

- Packaging the importer, `ffmpeg`, or a cron scheduler in this image.
- Scheduling imports or adding an importer Compose service.
- Splitting catalog JSON and thumbnails into separate roots.
- Providing a browser login system; the existing bearer token remains the only
  access control.
- Shipping a reverse proxy or TLS termination in the example stack.
- Building multi-arch images in the example workflow.

## Decisions

### Server-only image

Use a multi-stage image:

- **Build stage:** a pinned `rust:*` Debian bookworm image with
  `build-essential`; copy `Cargo.toml`, `Cargo.lock`, `src/`, `templates/`, and
  `assets/`; run `cargo build --locked --release --bin media-catalog-server`.
- **Runtime stage:** `debian:bookworm-slim` with `curl` for the health check.
  Copy only `media-catalog-server` and the entrypoint.

`templates/` and `assets/` must be copied because Askama and `include_str!`
embed them at compile time. The importer binary, `ffmpeg`, `ffprobe`, and
supercronic are intentionally absent.

**Alternative considered:** an all-in-one image with importer and cron was
rejected because the importer is handled separately. A slim runtime keeps the
server image smaller and avoids shipping media tools to the serving host.

### Entrypoint behavior

The entrypoint defaults to:

```sh
exec media-catalog-server --config /etc/media-catalog/media-catalog.toml
```

Any arguments passed to the container replace that default command and are
forwarded to the server. This keeps Compose thin while still allowing a manual
server invocation.

The health check is a small script in the image. It adds
`Authorization: Bearer` only when `MEDIA_CATALOG_TOKEN` is non-empty, avoiding
fragile Compose escaping and keeping the token out of the command line.

### Mount contract

| Mount | Container path | Server |
| --- | --- | --- |
| server config | `/etc/media-catalog/media-catalog.toml` | `ro` |
| media root | `/media` | `ro` |
| data root (`catalog/` + `thumbs/`) | `/data` | `ro` |
| state root | not mounted | absent |

The host-side names in the example are `./config`, `./media`, and `./data`. The
state directory remains on the host or with the separate importer deployment.

The example config uses absolute container paths:

```toml
media_root = "/media"
data_root = "/data"
bind = "0.0.0.0:8080"
```

The server bind must change from the loopback default because container
loopback is not reachable from the host. Compose publishes
`127.0.0.1:8080:8080` by default; a reverse proxy stack can instead publish
nothing and join the internal network.

### Users and permissions

The image declares a non-root runtime user (`10001:10001`). Compose overrides it
with `PUID`/`PGID` from `.env` (`1000:1000` by default) so the host media and
data directories are readable by the container on Linux.

The server container uses `read_only: true`, `no-new-privileges`,
`cap_drop: ALL`, and a small `/tmp` tmpfs. All catalog and media mounts are
read-only. No state directory is mounted.

Host directories must exist and allow read access for the configured UID/GID.
Docker Desktop's UID mapping is less predictable than a Linux host; the docs
will call this out.

### External importer independence

The importer runs outside this Compose stack: the existing host binary, a host
cron/systemd timer, or a future separate importer deployment. It continues to
need `ffmpeg`, write access to `data/` and `state/`, and `build.lock` protection.

The server has no dependency on the importer process. It can start before the
first catalog exists, serves the embedded bootstrap response, and reloads when a
complete `catalog/index.json` appears. This is the same snapshot behavior as a
host deployment; the container only changes where the server runs.

### Token handling

The token is optional and server-only. Compose passes
`MEDIA_CATALOG_TOKEN: ${MEDIA_CATALOG_TOKEN:-}`. An empty value disables access
control, preserving the current behavior for loopback or reverse-proxy
deployments.

Because auth wraps every route, including `/healthz`, the image ships a
token-aware health check script. The token is never written to the image or
configuration file.

**Alternative considered:** putting the token in the config file or image is
rejected because the server deliberately reads it only from the environment.

### Compose stack

The example stack has one service:

- `server`: published port, `/media` and `/data` read-only, config read-only,
  no state mount, token-aware health check.

There is no importer service and no cron schedule. The documentation explains
that a catalog must be produced by the separate import process before the server
can render pages.

### Build context and reproducibility

Add `.dockerignore` excluding `.git`, `target/`, `media/`, `data/`, `state/`,
and other local artifacts. Use `cargo build --locked` and pin the Rust base
image. The runtime layer installs only the packages the server needs.

## Risks / Trade-offs

- **No importer in the image** → operators must keep a separate import process
  with `ffmpeg`; document this prominently so an empty server does not look
  broken.
- **Read permission mismatch** → document `PUID`/`PGID` and host directory
  permissions; the server fails to serve missing or unreadable mounts.
- **Initial catalog absent** → `/healthz` reports `503` until the external
  importer publishes; Docker does not restart on unhealthy.
- **Token blocks browsers** → keep the token empty for loopback/trusted
  deployments, or inject the header at a TLS proxy. Document that this is not a
  login system.
- **Config uses host paths by mistake** → use absolute container paths in the
  shipped example and document the working-directory rule.
- **Import/server UID mismatch** → the importer must write files readable by the
  server UID; document a shared group or matching ownership.
- **Image does not include media tools** → server deployment remains small; any
  future transcoding feature would need its own image or tooling decision.
- **No multi-arch example** → the build follows the host architecture; buildx
  support can be added without changing the specs.

## Migration Plan

1. Build the server image and start it with the existing `media/` and `data/`
   directories mounted read-only.
2. Keep the existing import process and state directory unchanged; run it as
   before.
3. Start the Compose server and verify `/healthz` once the catalog exists.
4. Roll back by stopping the container and running the existing host server
   binary; no data migration is required.

## Open Questions

- Whether a separate importer image and scheduler are added later is deferred;
  this change only defines the server image and its deployment contract.
