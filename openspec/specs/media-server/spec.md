# Media Server Specification

## Purpose

Defines a read-only HTTP service that delivers the generated catalog and streams original media from a host that can access the media root.

## Requirements

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

The service SHALL emit validators for catalog JSON, rendered catalog pages, embedded assets, and originals, and SHALL honor conditional requests, while allowing immutable caching for versioned thumbnails.

#### Scenario: Unchanged original
- **WHEN** a client revalidates a media file with a matching `If-None-Match` or `If-Modified-Since`
- **THEN** the service responds with `304` and no body

#### Scenario: Versioned thumbnail
- **WHEN** a thumbnail URL carrying a version query is requested
- **THEN** the service serves it with a long-lived immutable cache policy

#### Scenario: Catalog revalidation
- **WHEN** a client requests catalog JSON or a rendered catalog page without an explicit version
- **THEN** the service allows revalidation rather than indefinite caching

#### Scenario: Rendered page unchanged
- **WHEN** a client revalidates a rendered catalog page with an ETag matching the current catalog snapshot and route
- **THEN** the service responds with `304` and no body

#### Scenario: Embedded asset revalidation
- **WHEN** a client revalidates an embedded shell asset
- **THEN** the service responds according to the asset's validator without reading the catalog tree

### Requirement: Keep the service read-only

The service SHALL NOT write to the data or media roots, SHALL NOT open or require the state root, SHALL NOT accept uploads, and SHALL NOT expose directory listings.

#### Scenario: Write method
- **WHEN** a client sends `POST`, `PUT`, or `DELETE`
- **THEN** the service rejects the method without changing either root

#### Scenario: Directory request
- **WHEN** a client requests a directory under `/media/`
- **THEN** the service returns `404` or `403` and does not enumerate its contents

#### Scenario: State root not opened
- **WHEN** the service serves catalog or media requests
- **THEN** it does not open `state.sqlite` or `build.lock` from any configured location

### Requirement: Fail clearly when the media root is unavailable
The service SHALL detect an unavailable or unmounted media root and report it distinctly from a missing file.

#### Scenario: Media mount missing
- **WHEN** the configured media root is absent or no longer a directory
- **THEN** `/media/` requests respond with `503` rather than a misleading `404`

#### Scenario: Media root restored
- **WHEN** the media root becomes available again
- **THEN** subsequent `/media/` requests are served normally without restarting the service

### Requirement: Control binding and access

The service SHALL resolve its bind address from `--bind`, then `bind` in the shared configuration file, and otherwise default to a loopback address. Access control SHALL be enabled only by a non-empty `MEDIA_CATALOG_TOKEN` environment variable.

#### Scenario: Default binding
- **WHEN** the service starts without a bind address in either flags or the configuration file
- **THEN** it listens only on a loopback address

#### Scenario: Configured binding
- **WHEN** the configuration file declares `bind = "0.0.0.0:8080"` and no `--bind` flag is given
- **THEN** the service listens on that address

#### Scenario: Command-line binding overrides configuration
- **WHEN** the configuration file declares a bind address and `--bind` is also given
- **THEN** the service listens on the flag address

#### Scenario: Missing credential
- **WHEN** access control is enabled and a request omits a valid credential
- **THEN** the service responds with `401`

#### Scenario: Valid credential
- **WHEN** access control is enabled and a request presents a valid credential
- **THEN** the request is served normally

#### Scenario: Token only from the environment
- **WHEN** `MEDIA_CATALOG_TOKEN` is unset or empty
- **THEN** access control is disabled regardless of the configuration file contents

### Requirement: Limit concurrent media streams

The service SHALL cap simultaneous `/media/` streams at `max_streams`, resolved from `--max-streams`, then `max_streams` in the shared configuration file, otherwise 8, and SHALL reject values below 1.

#### Scenario: Default stream limit
- **WHEN** neither source configures `max_streams`
- **THEN** the service admits up to eight concurrent media streams

#### Scenario: Configured stream limit
- **WHEN** the configuration file declares `max_streams = 2`
- **THEN** the service admits at most two concurrent media streams

#### Scenario: Command-line stream limit overrides configuration
- **WHEN** the configuration file declares `max_streams` and `--max-streams` is also given
- **THEN** the flag value is used

#### Scenario: Stream limit reached
- **WHEN** a media request arrives while the stream limit is already reached
- **THEN** the service responds with `503` and does not open the file

#### Scenario: Zero stream limit rejected
- **WHEN** `max_streams` resolves to `0`
- **THEN** startup fails with a configuration error before binding the listener

### Requirement: Shut down gracefully
The service SHALL stop accepting new connections on `SIGINT` or `SIGTERM`, allow in-flight responses to finish, and exit cleanly.

#### Scenario: Interrupt during transfer
- **WHEN** the service receives `SIGINT` while streaming a media response
- **THEN** it stops accepting new requests, drains the active response, and exits without a panic

#### Scenario: Repeated interrupt
- **WHEN** a second `SIGINT` arrives during shutdown
- **THEN** the service exits promptly without leaving the listener bound

### Requirement: Serve the embedded application shell and assets

The service SHALL serve the search page, viewer script, and browser assets embedded in the binary, independently of whether a catalog has been published.

#### Scenario: Search shell before catalog
- **WHEN** a client requests `/search.html` before any catalog has been published
- **THEN** the service returns the embedded search shell with an appropriate content type

#### Scenario: Embedded assets
- **WHEN** a client requests `/assets/site.css`, `/assets/search.js`, or the viewer script
- **THEN** the service returns the embedded asset with the correct content type and without reading the catalog tree

#### Scenario: Viewer asset before catalog
- **WHEN** a client requests the viewer script before any catalog has been published
- **THEN** the service returns it with a content validator rather than a catalog error

#### Scenario: Root bootstrap without catalog
- **WHEN** a client requests `/` and no published catalog is available
- **THEN** the service returns a bootstrap page that indicates the catalog is not yet available instead of a directory listing or an empty response

### Requirement: Render catalog pages from published JSON

The service SHALL render the root years index, the year months page, and the month item grid from the published JSON catalog on request, without requiring pre-generated HTML files.

#### Scenario: Root page
- **WHEN** a client requests `/` and a catalog is published
- **THEN** the service renders the years index from the catalog JSON

#### Scenario: Year and month pages
- **WHEN** a client requests `/{year}/` or `/{year}/{month}/` for a year or month present in the catalog
- **THEN** the service renders that page from the catalog JSON without directory listings

#### Scenario: Missing year or month
- **WHEN** a client requests a year or month that is not present in the catalog
- **THEN** the service responds with `404`

#### Scenario: Pages without JavaScript
- **WHEN** JavaScript is unavailable in the browser
- **THEN** the rendered catalog pages remain browsable and link to thumbnails and originals

### Requirement: Serve catalog data for client-side search

The service SHALL serve `catalog/index.json` and the referenced year shards so the client-side search page can load and filter the catalog without a backend query endpoint.

#### Scenario: Catalog index
- **WHEN** a client requests `/catalog/index.json`
- **THEN** the service returns the generated index with an appropriate JSON content type

#### Scenario: Year shard
- **WHEN** a client requests `/catalog/{year}.json` for a published year
- **THEN** the service returns the generated year shard with an appropriate JSON content type

#### Scenario: Catalog unavailable
- **WHEN** a client requests catalog JSON before a catalog has been published
- **THEN** the service responds with `404` or `503` and does not enumerate the catalog directory

### Requirement: Serve generated thumbnails

The service SHALL serve thumbnails from the generated thumbnail tree and SHALL keep versioned thumbnail URLs independently cacheable.

#### Scenario: Thumbnail request
- **WHEN** a client requests a generated thumbnail under `/thumbs/`
- **THEN** the service returns the thumbnail bytes with an image content type

#### Scenario: Versioned thumbnail
- **WHEN** a thumbnail URL carries a version query
- **THEN** the service serves it with a long-lived immutable cache policy

#### Scenario: Missing thumbnail
- **WHEN** a client requests a thumbnail that does not exist
- **THEN** the service responds with `404` and does not enumerate directories

### Requirement: Load catalog snapshots atomically

The service SHALL build an immutable catalog snapshot from `catalog/index.json` and its referenced shards, and SHALL replace the serving snapshot only after a complete load succeeds.

#### Scenario: Initial snapshot
- **WHEN** the service receives a catalog request and a published catalog exists
- **THEN** it loads the index and referenced shards into a snapshot and renders from that snapshot

#### Scenario: Catalog update
- **WHEN** the catalog index changes on disk
- **THEN** the service loads a new snapshot and subsequent requests use it without requiring a restart

#### Scenario: Partial publication
- **WHEN** a catalog reload cannot read every referenced shard
- **THEN** the service keeps serving the previous snapshot and does not expose a partial catalog

#### Scenario: Index as commit point
- **WHEN** the generator publishes JSON
- **THEN** `catalog/index.json` is treated as the commit marker for the shards it references

### Requirement: Start before a catalog is published

The service SHALL start when the catalog root or catalog JSON is absent, and SHALL report catalog readiness separately from media readiness.

#### Scenario: No catalog at startup
- **WHEN** the service starts without a catalog root or published JSON
- **THEN** the shell and assets remain available, `/healthz` reports catalog unavailable, and the service does not exit

#### Scenario: Catalog appears later
- **WHEN** the generator publishes a catalog after the service has started
- **THEN** catalog pages become available without restarting the service

#### Scenario: Media root missing
- **WHEN** the catalog is available but the media root is missing
- **THEN** catalog pages remain servable and `/media/` responds with `503`

### Requirement: Read published catalog from the data root

The service SHALL resolve published catalog JSON and thumbnails directly under the configured data root and SHALL NOT derive a `public/` subtree or require a state root.

#### Scenario: Catalog index
- **WHEN** the data root is `./data` and a catalog is published
- **THEN** a request for `/catalog/index.json` is served from `./data/catalog/index.json`

#### Scenario: Thumbnails
- **WHEN** the data root is `./data`
- **THEN** a request under `/thumbs/` is served from `./data/thumbs/`

#### Scenario: Missing data root
- **WHEN** the service starts before the data root contains `catalog/index.json`
- **THEN** it starts normally, reports the catalog as unavailable, and does not create the data root
