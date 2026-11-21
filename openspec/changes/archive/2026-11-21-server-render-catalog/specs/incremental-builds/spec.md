# Spec Delta

## ADDED Requirements

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

## MODIFIED Requirements

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
