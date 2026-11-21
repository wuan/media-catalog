# Spec Delta

## Purpose

Defines a read-only HTTP service that delivers the generated catalog and streams original media from a host that can access the media root.

## ADDED Requirements

### Requirement: Serve the generated catalog tree
The service SHALL serve the generated `public/` tree at `/`, including HTML pages, JSON shards, assets, and thumbnails, without requiring a database connection at runtime.

#### Scenario: Root page
- **WHEN** a client requests `/`
- **THEN** the service returns the generated `index.html`

#### Scenario: Year and month pages
- **WHEN** a client requests `/{year}/` or `/{year}/{month}/`
- **THEN** the service returns that generated page without directory listings

#### Scenario: JSON shards and assets
- **WHEN** a client requests `/catalog/index.json`, `/catalog/{year}.json`, or a file under `/assets/`
- **THEN** the service returns the generated file with an appropriate content type

### Requirement: Stream original media read-only
The service SHALL stream files from the configured media root at `/media/` without modifying them, and SHALL return the correct content type for supported media extensions.

#### Scenario: Original image
- **WHEN** a client requests `/media/2024/05/photo.jpg`
- **THEN** the service streams the original bytes with an image content type

#### Scenario: Missing original
- **WHEN** a client requests a path that does not exist under the media root
- **THEN** the service responds with `404` and no filesystem listing

### Requirement: Support byte-range requests for media
The service SHALL advertise and honor single byte-range requests for `/media/` responses so clients can seek within large videos.

#### Scenario: Range request
- **WHEN** a client requests `/media/2024/05/clip.mp4` with `Range: bytes=1000-1999`
- **THEN** the service responds with `206`, `Content-Range: bytes 1000-1999/...`, and exactly those bytes

#### Scenario: Unsatisfiable range
- **WHEN** a client requests a range outside the file size
- **THEN** the service responds with `416` and a `Content-Range` describing the valid size

#### Scenario: HEAD request
- **WHEN** a client sends `HEAD` for a media file
- **THEN** the service returns the same headers as `GET` without a response body

### Requirement: Reject unsafe paths
The service SHALL reject path traversal, encoded traversal, and symlink escape attempts, and SHALL keep all served files inside the configured roots.

#### Scenario: Parent-directory traversal
- **WHEN** a client requests a path containing `..` segments
- **THEN** the service responds with `400` or `404` and does not read outside the root

#### Scenario: Encoded traversal
- **WHEN** a client requests percent-encoded traversal such as `%2e%2e%2f`
- **THEN** the service decodes once, rejects the traversal, and does not read outside the root

#### Scenario: Symlink escape
- **WHEN** a path resolves through a symlink to a location outside the media root
- **THEN** the service refuses to serve it

### Requirement: Use conditional caching for catalog and media
The service SHALL emit validators for catalog files and originals and SHALL honor conditional requests, while allowing immutable caching for versioned thumbnails.

#### Scenario: Unchanged original
- **WHEN** a client revalidates a media file with a matching `If-None-Match` or `If-Modified-Since`
- **THEN** the service responds with `304` and no body

#### Scenario: Versioned thumbnail
- **WHEN** a thumbnail URL carrying a version query is requested
- **THEN** the service serves it with a long-lived immutable cache policy

#### Scenario: Catalog revalidation
- **WHEN** a client requests generated HTML or JSON without an explicit version
- **THEN** the service allows revalidation rather than indefinite caching

### Requirement: Keep the service read-only
The service SHALL NOT write to the catalog or media roots, accept uploads, or expose directory listings.

#### Scenario: Write method
- **WHEN** a client sends `POST`, `PUT`, or `DELETE`
- **THEN** the service rejects the method without changing either root

#### Scenario: Directory request
- **WHEN** a client requests a directory under `/media/`
- **THEN** the service returns `404` or `403` and does not enumerate its contents

### Requirement: Fail clearly when the media root is unavailable
The service SHALL detect an unavailable or unmounted media root and report it distinctly from a missing file.

#### Scenario: Media mount missing
- **WHEN** the configured media root is absent or no longer a directory
- **THEN** `/media/` requests respond with `503` rather than a misleading `404`

#### Scenario: Media root restored
- **WHEN** the media root becomes available again
- **THEN** subsequent `/media/` requests are served normally without restarting the service

### Requirement: Control binding and access
The service SHALL accept a configurable bind address and optional access credential, and SHALL default to a private bind address.

#### Scenario: Default binding
- **WHEN** the service starts without an explicit bind address
- **THEN** it listens only on a loopback address

#### Scenario: Missing credential
- **WHEN** access control is enabled and a request omits a valid credential
- **THEN** the service responds with `401`

#### Scenario: Valid credential
- **WHEN** access control is enabled and a request presents a valid credential
- **THEN** the request is served normally

### Requirement: Shut down gracefully
The service SHALL stop accepting new connections on `SIGINT` or `SIGTERM`, allow in-flight responses to finish, and exit cleanly.

#### Scenario: Interrupt during transfer
- **WHEN** the service receives `SIGINT` while streaming a media response
- **THEN** it stops accepting new requests, drains the active response, and exits without a panic

#### Scenario: Repeated interrupt
- **WHEN** a second `SIGINT` arrives during shutdown
- **THEN** the service exits promptly without leaving the listener bound
