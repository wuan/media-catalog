# Spec Delta

## MODIFIED Requirements

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
