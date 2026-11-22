# Spec Delta

## MODIFIED Requirements

### Requirement: Define events in a content file

The generator SHALL read an optional authored events file whose path is declared by `events_file` in `media-catalog.toml`, or by the `--events` flag. The CLI flag SHALL override the configured path. Each event SHALL declare a stable `slug`, `title`, inclusive `start` and `end` timestamps in `YYYY-MM-DD HH:MM` form, and MAY declare a `description`.

#### Scenario: Default content file
- **WHEN** `media-catalog.toml` declares `events_file = "path/to/events.toml"` and no `--events` flag is given
- **THEN** the generator loads the events from that path

#### Scenario: Explicit content file
- **WHEN** `--events path/to/events.toml` is given while `media-catalog.toml` also declares `events_file`
- **THEN** the generator loads the events from the `--events` path

#### Scenario: No content file
- **WHEN** neither `events_file` nor `--events` is set
- **THEN** the build proceeds without events and publishes a catalog without event data

#### Scenario: Missing explicit file
- **WHEN** `--events` names a file that does not exist
- **THEN** the generator fails before scanning and reports the missing path

#### Scenario: Missing configured file
- **WHEN** `events_file` names a file that does not exist
- **THEN** the generator fails before scanning and reports the missing path

#### Scenario: Relative configured path
- **WHEN** `events_file` is a relative path
- **THEN** the generator resolves it against the current directory, like the storage root settings
