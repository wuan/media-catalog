# Spec Delta

## ADDED Requirements

### Requirement: Serve the embedded application shell and assets

The service SHALL serve the search page and browser assets embedded in the binary, independently of whether a catalog has been published.

#### Scenario: Search shell before catalog
- **WHEN** a client requests `/search.html` before any catalog has been published
- **THEN** the service returns the embedded search shell with an appropriate content type

#### Scenario: Embedded assets
- **WHEN** a client requests `/assets/site.css` or `/assets/search.js`
- **THEN** the service returns the embedded asset with the correct content type and without reading the catalog tree

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

## MODIFIED Requirements

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

## REMOVED Requirements

### Requirement: Serve the generated catalog tree

**Reason**: The service no longer serves a pre-generated HTML tree. It renders catalog pages from JSON and serves the shell and assets embedded in the binary.

**Migration**: The existing `/`, `/{year}/`, `/{year}/{month}/`, `/catalog/*`, `/assets/*`, and `/thumbs/*` routes remain available. Catalog and shell clients do not need URL changes, but deployments no longer require generated HTML or separately deployed asset files.
