# Spec Delta

## ADDED Requirements

### Requirement: Support an events-only run setting

The generator SHALL accept an `events_only` setting resolved from the `--events-only` flag first and the `events_only` key in `media-catalog.toml` second, defaulting to off. When enabled, the generator SHALL republish the catalog JSON from persisted build state without scanning, probing, or generating thumbnails. The server SHALL accept and ignore the key like other generator-only keys.

#### Scenario: Flag enables events-only mode
- **WHEN** the generator is started with `--events-only`
- **THEN** it republishes the catalog from state without scanning the media root

#### Scenario: TOML key enables events-only mode
- **WHEN** `media-catalog.toml` sets `events_only = true` and no flag is given
- **THEN** the generator runs in events-only mode

#### Scenario: Flag wins over the file
- **WHEN** the configuration file sets `events_only = false` and `--events-only` is given
- **THEN** the generator runs in events-only mode

#### Scenario: Server ignores the key
- **WHEN** `media-catalog.toml` contains `events_only = true`
- **THEN** `media-catalog-server` starts normally and ignores the key

### Requirement: Relax media-root existence validation in events-only mode

In events-only mode the generator SHALL NOT require the configured media root to exist on disk, because the mode never reads it; all other validation SHALL still apply. Outside events-only mode the existence check SHALL remain in force.

#### Scenario: Unmounted media root
- **WHEN** the generator runs with `--events-only` and the configured media root does not exist
- **THEN** validation passes and the run proceeds from state

#### Scenario: Full build still validates existence
- **WHEN** the generator runs without `--events-only` and the configured media root does not exist
- **THEN** the run fails validation naming the media root
