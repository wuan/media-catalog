# Spec Delta

## MODIFIED Requirements

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
