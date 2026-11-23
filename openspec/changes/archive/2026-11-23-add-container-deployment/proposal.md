# Proposal

## Why

Running the catalog server currently requires either a Rust build on the host or
a hand-built binary plus its runtime dependencies. A server-only container image
makes the serving runtime reproducible and keeps it isolated from the host. The
importer remains a separate concern and is not packaged by this change.

## What Changes

- Add a multi-stage container image containing only `media-catalog-server` and
  the minimal runtime dependencies needed to serve the catalog.
- Add a container entrypoint that starts the server by default and forwards
  command-line arguments.
- Add an example Docker Compose deployment with three mounts: the server
  configuration file, the media archive, and the catalog/data root. The server
  does not mount the state root.
- Keep the existing storage contract: the server receives media and catalog data
  read-only and never reads or writes build state.
- Document image layout, permissions, mounts, token behavior, and how a catalog
  produced by an external import is picked up.
- Leave importer packaging, scheduling, and the importer's `ffmpeg` dependency
  outside this change. They continue to run separately from the server image.

## Capabilities

### New Capabilities

- `container-deployment`: server-only container image, entrypoint behavior,
  read-only mount contract, external-import independence, token handling, and the
  example Compose deployment.

### Modified Capabilities

<!-- No existing requirement changes. The storage-roots and configuration specs
     continue to describe the host binaries unchanged. -->

## Impact

- New packaging files: `Dockerfile`, `.dockerignore`, container entrypoint, and
  a server-only example `compose.yaml`.
- New deployment documentation under `docs/deploy/`.
- No Rust source or catalog format changes.
- The runtime image excludes the importer binary, `ffmpeg`, `ffprobe`, and any
  cron scheduler, so it is smaller than an all-in-one image.
- Operators must map container paths (`/media`, `/data`) in the container
  configuration; the external importer keeps its own configuration and state.
