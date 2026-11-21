# Spec Delta

## ADDED Requirements

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

## MODIFIED Requirements

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
