# Spec Delta

## ADDED Requirements

### Requirement: Republish events without scanning, probing, or thumbnails

When the generator runs in events-only mode, it SHALL load the persisted item records from the state database, rebuild the catalog, reattach the authored events, and republish the catalog JSON, without reading the media root, invoking external media tools, or generating thumbnails. The republished event metadata and membership SHALL match what a full run over unchanged inputs would publish.

#### Scenario: Quick event update
- **WHEN** the events file changes and the generator runs with `--events-only`
- **THEN** `catalog/index.json` and the year shards reflect the new event metadata and membership, and no scan, probe, or thumbnail work is performed

#### Scenario: Media root unmounted
- **WHEN** the generator runs with `--events-only` while the media root does not exist
- **THEN** the run succeeds and republishes the catalog from state

#### Scenario: External tools missing
- **WHEN** the generator runs with `--events-only` and ffmpeg/ffprobe are not on `PATH`
- **THEN** the run succeeds, because no probing or thumbnail work is needed

#### Scenario: No state database
- **WHEN** `--events-only` is given before any full build has created the state database
- **THEN** the generator exits non-zero with an error stating that a full build is required first, and publishes no catalog output

#### Scenario: Dry run reports without writing
- **WHEN** `--events-only --dry-run` is given
- **THEN** the generator validates the events file and reports the planned republish without writing catalog output or state

#### Scenario: Identical output to a full run
- **WHEN** an events-only run republishes unchanged events after a full build
- **THEN** the published JSON is byte-identical to the full build's output

#### Scenario: Invalid events still fail
- **WHEN** an events-only run reads an events file that fails validation
- **THEN** the generator exits non-zero before publishing and changes no catalog output

### Requirement: Report events-only runs

The run output of an events-only run SHALL identify the run as events-only, state that scanning, probing, and thumbnail generation were skipped, and report the number of JSON files republished plus each published event and its item count.

#### Scenario: Events-only summary
- **WHEN** an events-only run completes
- **THEN** the summary marks the run as events-only and lists republished JSON files and per-event counts
