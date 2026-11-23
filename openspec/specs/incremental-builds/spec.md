# Incremental Builds Specification

## Purpose

Defines the build state that keeps repeated runs fast and safe: persistent SQLite records, source fingerprints, resumable thumbnail work, output atomicity, concurrent-run protection, and orphan cleanup.

## Requirements

### Requirement: Persist build state in SQLite

The generator SHALL store build state in a SQLite database under the configured state root outside the published data tree, recording each item's relative path, source fingerprint, extracted metadata, and thumbnail status.

#### Scenario: First run
- **WHEN** the generator runs for the first time
- **THEN** it creates the state database and records every processed item

#### Scenario: State outside output
- **WHEN** the data root is served or backed up
- **THEN** the SQLite database is not part of the published files

#### Scenario: Explicit state root
- **WHEN** the state root is configured as `./state`
- **THEN** the generator creates or opens `./state/state.sqlite`

### Requirement: Skip unchanged items
The generator SHALL use a fingerprint derived from file size and modification time to decide whether an item needs thumbnail regeneration, and SHALL reuse existing thumbnails and cached metadata for unchanged items.

#### Scenario: Incremental run
- **WHEN** a catalog of 75,000 items is rerun with only 500 new files
- **THEN** only the new or changed items are decoded and thumbnailed

#### Scenario: Unchanged item metadata
- **WHEN** an item's fingerprint is unchanged
- **THEN** its stored metadata and thumbnail are reused without reopening the source

### Requirement: Resume interrupted builds
The generator SHALL record each thumbnail as completed only after it is written successfully, so a rerun after an interruption continues with unfinished work instead of starting over.

#### Scenario: Interrupted first build
- **WHEN** the process stops partway through a full build
- **THEN** rerunning the generator skips completed thumbnails and continues with the rest

#### Scenario: Failed thumbnail retried
- **WHEN** a thumbnail failed during a previous run but its source is unchanged
- **THEN** the next run retries that thumbnail rather than treating it as complete

### Requirement: Reflect deletions in generated output

The generator SHALL remove deleted items from regenerated JSON, and SHALL remove thumbnails that no longer belong to any catalog item when pruning is enabled. Server-rendered pages SHALL reflect the published JSON catalog.

#### Scenario: Deleted item
- **WHEN** an item is removed from the media tree and the generator runs again
- **THEN** it no longer appears in catalog JSON or server-rendered month pages

#### Scenario: Prune enabled
- **WHEN** pruning is enabled and a thumbnail has no corresponding catalog item
- **THEN** the orphan thumbnail is deleted from the thumbnail tree

#### Scenario: Prune disabled
- **WHEN** pruning is not enabled
- **THEN** orphan thumbnails may remain and no files outside the thumbnail tree are removed

### Requirement: Prevent concurrent builds

The generator SHALL acquire a build lock beside the state database in the state root and SHALL refuse to start when another build is already running against the same state root.

#### Scenario: Overlapping manual run
- **WHEN** a second build starts while the first still holds the lock
- **THEN** the second exits with a clear message and does not modify output

#### Scenario: Lock released after failure
- **WHEN** a build fails or is interrupted
- **THEN** the lock is released and a later run can proceed

#### Scenario: Lock location
- **WHEN** the state root is configured as `./state`
- **THEN** the lock file is `./state/build.lock`

### Requirement: Write output atomically

The generator SHALL write JSON shards and thumbnails through temporary files and atomic replacement so visitors never observe partially written output.

#### Scenario: Interrupted JSON write
- **WHEN** the generator is interrupted while writing a JSON shard
- **THEN** the previous complete shard remains in place or no partial shard is left behind

#### Scenario: Interrupted page write
- **WHEN** the generator is interrupted while publishing catalog output
- **THEN** the previous complete catalog remains in place or no partial output is left behind, and the runtime keeps serving the previous snapshot

#### Scenario: Thumbnail replacement
- **WHEN** a thumbnail is regenerated
- **THEN** the new file replaces the old one atomically

### Requirement: Recover from missing or forced state
The generator SHALL rebuild missing thumbnails when state is absent and SHALL provide a force mode that regenerates thumbnails even when fingerprints are unchanged.

#### Scenario: State database deleted
- **WHEN** the SQLite state file is removed
- **THEN** a subsequent run rebuilds the state and generates thumbnails as needed

#### Scenario: Force rebuild
- **WHEN** the generator runs in force mode
- **THEN** thumbnails are regenerated regardless of existing fingerprints

### Requirement: Publish the catalog index last

The generator SHALL write all referenced year shards before replacing `catalog/index.json`, so the runtime can treat the index as the commit point for a complete catalog snapshot.

#### Scenario: Successful publication
- **WHEN** the generator publishes a catalog update
- **THEN** every shard referenced by the new index is already written when the new index becomes visible

#### Scenario: Interrupted publication
- **WHEN** the generator is interrupted after writing some shards but before writing the new index
- **THEN** the previous index remains in place and the runtime continues serving the previous complete snapshot

#### Scenario: Runtime reload
- **WHEN** the runtime observes a new catalog index
- **THEN** it can load every referenced shard without encountering a partially published catalog

### Requirement: Protect build state during events-only runs

An events-only run SHALL NOT delete unseen state rows, reset item thumbnail status, prune orphan thumbnails, or trigger the mount-drop guard, because no scan observes the media tree. It SHALL still acquire the build lock before republishing output.

#### Scenario: State rows preserved
- **WHEN** an events-only run completes
- **THEN** every item row remains in the state database with its previous thumbnail status, metadata, and last-seen run

#### Scenario: Prune and cleanup suppressed
- **WHEN** the generator runs with `--events-only --prune`
- **THEN** no thumbnails are pruned and no state rows are deleted

#### Scenario: Lock still enforced
- **WHEN** an events-only run is in progress and a second build starts against the same state root
- **THEN** the second build exits with a clear message and does not modify output

#### Scenario: Mount-drop guard inactive
- **WHEN** the media root is unmounted and an events-only run executes
- **THEN** the run is not blocked and reports no mount-drop warning

#### Scenario: Next full run cleans up normally
- **WHEN** a full build runs after an events-only run
- **THEN** deletion detection and pruning behave exactly as if the events-only run had not happened

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

### Requirement: Retry failed probes on later runs

The generator SHALL probe an unchanged decodable image or video again on the next run when its previous probe failed, and SHALL keep doing so on each run until a probe succeeds. Once a probe succeeds, later runs with an unchanged fingerprint SHALL reuse the stored metadata without reopening the source.

#### Scenario: Recovery after the environment is fixed
- **WHEN** a JPEG's probe fails because the file cannot be read, and a later run with the same file size and modification time can read it
- **THEN** the later run probes it again and the published catalog records its upright width, height, and camera metadata

#### Scenario: Still unreadable
- **WHEN** a decodable item's probe failed on the previous run and the file is still unreadable
- **THEN** the run attempts the probe again, counts it as a probe failure, and keeps the item cataloged with its fallback capture time

#### Scenario: No retry after success
- **WHEN** a previously failed item was successfully probed and its fingerprint is unchanged
- **THEN** the next run reuses the stored metadata and does not reopen the source
