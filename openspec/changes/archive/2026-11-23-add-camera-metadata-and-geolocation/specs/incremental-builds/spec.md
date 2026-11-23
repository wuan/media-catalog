# Spec Delta

## ADDED Requirements

### Requirement: Migrate state for camera metadata

The generator SHALL upgrade a version 2 state database to version 3 by adding storage for lens settings and coordinates and clearing cached probe metadata once, so existing items are probed again while completed thumbnails are kept.

#### Scenario: Upgrade from version 2
- **WHEN** the generator opens a version 2 state database
- **THEN** it adds the new metadata storage, clears cached probe fields once, and writes catalog output using the new fields

#### Scenario: Thumbnails preserved
- **WHEN** the version 2 database already records successful thumbnails
- **THEN** those thumbnails are reused and are not regenerated because of the migration

#### Scenario: One-time re-probe
- **WHEN** the first run after the upgrade probes every existing item
- **THEN** a later run with unchanged sources reuses the cached metadata and does not reopen those files

#### Scenario: HEIC metadata filled
- **WHEN** an unchanged HEIC item is probed after the upgrade
- **THEN** its container metadata is recorded even though its thumbnail remains unavailable

#### Scenario: Unsupported schema rejected
- **WHEN** the state database declares a schema version other than the supported version
- **THEN** the generator exits non-zero with an error naming the schema version

### Requirement: Reuse metadata across location publication changes

The generator SHALL read lens settings and coordinates from persisted state, so enabling or disabling location publication republishes the catalog without re-probing unchanged sources.

#### Scenario: Enable location publication
- **WHEN** location publication is enabled for a version 3 state database
- **THEN** the catalog is republished with coordinates from state and unchanged sources are not probed again

#### Scenario: Disable location publication
- **WHEN** location publication is disabled for a version 3 state database
- **THEN** the catalog is republished without coordinates from state and unchanged sources are not probed again

#### Scenario: Events-only mode
- **WHEN** the generator republishes events from state
- **THEN** camera settings and published coordinates come from the same persisted rows without probing
