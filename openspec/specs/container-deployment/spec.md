# Container Deployment Specification

## Purpose

Packages the catalog runtime server into a container image and defines the
read-only mounts, permissions, and deployment behavior used to serve a catalog
produced by a separate importer.

## Requirements

### Requirement: Image provides only the server and its runtime dependencies

The image SHALL provide the `media-catalog-server` executable and SHALL NOT
include the `media-catalog` importer executable, `ffmpeg`, `ffprobe`, or a cron
scheduler.

#### Scenario: Server executable is present
- **WHEN** a container is started with `media-catalog-server --help`
- **THEN** the server prints usage and exits successfully

#### Scenario: Importer is absent
- **WHEN** the image is inspected for the importer executable
- **THEN** no `media-catalog` binary is present

#### Scenario: Media tools are absent
- **WHEN** the container is asked to run `ffmpeg` or `ffprobe`
- **THEN** neither tool is available on `PATH`

### Requirement: Container starts the server by default

The image SHALL start `media-catalog-server` when no command is given and SHALL
forward any supplied arguments to the server.

#### Scenario: Default start
- **WHEN** the container starts without an argument
- **THEN** it runs `media-catalog-server`

#### Scenario: Argument override
- **WHEN** the container is started with server flags such as `--bind`
- **THEN** the flags are passed to `media-catalog-server`

#### Scenario: Invalid server argument
- **WHEN** the container receives an argument the server rejects
- **THEN** the server exits non-zero with its normal argument error

### Requirement: Deployment uses a read-only server mount contract

The container deployment SHALL mount the server configuration file read-only,
the media root read-only at `/media`, and the data root at `/data` containing
both `catalog/` and `thumbs/`. It SHALL NOT mount the state root.

#### Scenario: Server mounts
- **WHEN** the server container starts
- **THEN** it has the configuration file, `/media`, and `/data`, and no `/state`

#### Scenario: Catalog and thumbnails share one root
- **WHEN** the data mount is inspected
- **THEN** it contains both `catalog/` and `thumbs/`

#### Scenario: State root stays with the importer
- **WHEN** the server runs while an external importer writes build state
- **THEN** the server serves catalog data without opening `state.sqlite` or
  `build.lock`

### Requirement: Container configuration uses container paths

The deployment SHALL configure the server's media and data roots as `/media`
and `/data` and SHALL bind the server beyond loopback so the published port is
reachable.

#### Scenario: Host paths are not used in the container config
- **WHEN** the container's `media-catalog.toml` is read inside the container
- **THEN** its media and data roots resolve to the mounted container directories

#### Scenario: Published server port
- **WHEN** the server container publishes port 8080
- **THEN** requests reach the server from outside the container

### Requirement: Server runs read-only as a non-root user

The server container SHALL run as a numeric non-root user, SHALL use a read-only
root filesystem, and SHALL have no writable catalog, media, or state mounts.

#### Scenario: Data stays read-only
- **WHEN** the server attempts to open a file under `/data` for writing
- **THEN** the write fails while normal reads continue

#### Scenario: Originals stay read-only
- **WHEN** the server attempts to modify a file under `/media`
- **THEN** the write fails while streaming reads continue

#### Scenario: Non-root execution
- **WHEN** the server container starts
- **THEN** its process runs with a numeric non-root user

### Requirement: Server is independent of the importer

The server SHALL start without a catalog, SHALL serve its bootstrap behavior
until a catalog is published, and SHALL pick up a catalog published by an
external importer without a restart.

#### Scenario: Start before first catalog
- **WHEN** the server starts while `/data/catalog/index.json` does not exist
- **THEN** it serves the embedded bootstrap response and reports catalog
  readiness as unavailable

#### Scenario: External import publishes a catalog
- **WHEN** a separate importer writes a complete catalog into the mounted data
  root
- **THEN** the running server begins serving rendered catalog pages without a
  restart

#### Scenario: No importer dependency
- **WHEN** no importer container or process is running
- **THEN** the server keeps serving an already published catalog

### Requirement: Access token stays out of the image

The image and configuration file SHALL NOT contain an access token. A server
token SHALL be supplied at runtime through `MEDIA_CATALOG_TOKEN`.

#### Scenario: No baked-in token
- **WHEN** the image is inspected without runtime environment
- **THEN** no access token is present

#### Scenario: Token-enabled health check
- **WHEN** the server runs with a non-empty token
- **THEN** the container health check sends `Authorization: Bearer` and receives
  a readiness response instead of `401`

#### Scenario: Token disabled
- **WHEN** the server runs with an empty or unset token
- **THEN** unauthenticated requests are served normally

### Requirement: Example Compose deployment is provided

The project SHALL include an example Compose deployment that builds the
server-only image, starts one server service, wires the read-only mounts,
permissions, runtime environment, and health check, and documents how an
external import supplies the catalog.

#### Scenario: Compose configuration validates
- **WHEN** an operator runs the Compose configuration check with the example
  environment file
- **THEN** the configuration is accepted and lists one server service

#### Scenario: Catalog supplied externally
- **WHEN** the operator follows the documented external import procedure
- **THEN** the Compose server serves the published catalog without adding an
  importer service

#### Scenario: Server-only stack
- **WHEN** the Compose stack is started
- **THEN** only the server container runs; no importer or cron container is part
  of the stack

### Requirement: Build context excludes runtime data

The image build SHALL exclude the media archive, catalog data, state directory,
build output, and version-control metadata from the build context.

#### Scenario: Large archive not sent to the builder
- **WHEN** the image is built from the repository root
- **THEN** the build context does not include the media, data, state, or target
  directories

#### Scenario: Clean build without local catalog
- **WHEN** the image is built on a machine with no catalog data
- **THEN** the build succeeds without requiring runtime directories
