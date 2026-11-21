# Incremental Builds Specification

## Purpose

Defines the build state that keeps repeated runs fast and safe: persistent SQLite records, source fingerprints, resumable thumbnail work, output atomicity, concurrent-run protection, and orphan cleanup.

## Requirements

### Requirement: Persist build state in SQLite
The generator SHALL store build state in a SQLite database outside the deployable web tree, recording each item's relative path, source fingerprint, extracted metadata, and thumbnail status.

#### Scenario: First run
- **WHEN** the generator runs for the first time
- **THEN** it creates the state database and records every processed item

#### Scenario: State outside output
- **WHEN** the web tree is deployed
- **THEN** the SQLite database is not part of the deployable files

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
The generator SHALL remove deleted items from regenerated pages and JSON, and SHALL remove thumbnails that no longer belong to any catalog item when pruning is enabled.

#### Scenario: Deleted item
- **WHEN** an item is removed from the media tree and the generator runs again
- **THEN** it no longer appears in month pages or JSON

#### Scenario: Prune enabled
- **WHEN** pruning is enabled and a thumbnail has no corresponding catalog item
- **THEN** the orphan thumbnail is deleted from the web tree

#### Scenario: Prune disabled
- **WHEN** pruning is not enabled
- **THEN** orphan thumbnails may remain and no files outside the thumbnail tree are removed

### Requirement: Prevent concurrent builds
The generator SHALL acquire a build lock and SHALL refuse to start when another build is already running against the same state.

#### Scenario: Overlapping manual run
- **WHEN** a second build starts while the first still holds the lock
- **THEN** the second exits with a clear message and does not modify output

#### Scenario: Lock released after failure
- **WHEN** a build fails or is interrupted
- **THEN** the lock is released and a later run can proceed

### Requirement: Write output atomically
The generator SHALL write pages and thumbnails through temporary files and atomic replacement so visitors never observe partially written output.

#### Scenario: Interrupted page write
- **WHEN** the generator is interrupted while writing a page
- **THEN** the previous complete page remains in place or no partial page is left behind

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
