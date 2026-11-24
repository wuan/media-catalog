# configuration Specification

## Purpose

Defines the shared `media-catalog.toml` deployment contract used by both binaries: file discovery, key schema, precedence with command-line flags, relative-path resolution, and startup validation.

## Requirements

### Requirement: Read one shared configuration file

Both binaries SHALL read the same optional `media-catalog.toml`. An explicit `--config PATH` SHALL select that file and SHALL fail when it is absent or invalid. Without `--config`, each binary SHALL use `./media-catalog.toml` from the working directory when that file exists, and SHALL proceed without a file when it does not.

#### Scenario: Server reads the working-directory file
- **WHEN** `media-catalog-server` starts with no `--config` and the working directory contains `media-catalog.toml`
- **THEN** it reads that file before resolving its settings

#### Scenario: Explicit configuration file missing
- **WHEN** either binary is given `--config` pointing to a path that does not exist
- **THEN** it exits non-zero with an error naming the missing configuration file

#### Scenario: No configuration file present
- **WHEN** no explicit `--config` is given and `./media-catalog.toml` does not exist
- **THEN** the binary proceeds using command-line values and defaults without an error

### Requirement: Resolve settings from CLI, file, then default

Each setting SHALL resolve from its command-line flag first, the configuration file second, and a built-in default only where one exists. Storage roots SHALL have no default and SHALL fail when neither source provides them.

#### Scenario: Server roots from the file
- **WHEN** `media-catalog.toml` declares `media_root` and `data_root` and the server is started without root flags
- **THEN** the server uses the configured roots

#### Scenario: Command line overrides the file
- **WHEN** the configuration file declares a value and the matching flag is also given
- **THEN** the flag value wins for that setting while other settings still come from the file

#### Scenario: Missing storage root
- **WHEN** neither the flag nor the configuration file provides a required storage root
- **THEN** the binary exits non-zero with an error naming both the flag and its TOML key

#### Scenario: Built-in defaults
- **WHEN** neither source provides `bind`, `max_streams`, or `jobs`
- **THEN** the applicable binary uses its documented built-in default for that setting

### Requirement: Share one key schema across binaries

Both binaries SHALL parse the same key schema and SHALL reject unknown keys. Each binary SHALL accept and ignore keys that only the other binary consumes.

#### Scenario: Server accepts generator keys
- **WHEN** `media-catalog.toml` contains `state_root`, `jobs`, `force`, `prune`, `dry_run`, `quiet`, `allow_mount_drop`, `events_file`, or URL prefixes
- **THEN** `media-catalog-server` starts normally and ignores the keys it does not use

#### Scenario: Generator accepts server keys
- **WHEN** `media-catalog.toml` contains `bind` or `max_streams`
- **THEN** the generator runs normally and ignores those keys

#### Scenario: Unknown key rejected
- **WHEN** the configuration file contains a key outside the shared schema
- **THEN** the binary exits non-zero with an error naming the configuration file and the unknown key

### Requirement: Resolve relative configured paths against the working directory

Relative paths in the configuration file SHALL resolve against the process working directory, not the directory containing the configuration file.

#### Scenario: Relative roots in a file elsewhere
- **WHEN** `--config` points outside the working directory and the file declares `data_root = "./data"`
- **THEN** the data root resolves to `./data` under the working directory

#### Scenario: Relative root from the default file
- **WHEN** `./media-catalog.toml` declares a relative media root
- **THEN** both binaries resolve it against the working directory

### Requirement: Validate configuration before doing work

The binaries SHALL validate resolved settings before the generator writes output or opens build state and before the server binds a listener. Invalid values SHALL produce a non-zero exit with an error naming the setting.

#### Scenario: Invalid bind address
- **WHEN** the configuration file or `--bind` supplies an address that is not a valid socket address
- **THEN** the server exits non-zero without binding a listener

#### Scenario: Zero stream limit
- **WHEN** `max_streams` resolves to `0`
- **THEN** the server exits non-zero with an error naming `--max-streams` or `max_streams`

#### Scenario: Invalid file content
- **WHEN** the configuration file cannot be parsed
- **THEN** the binary exits non-zero with an error naming the file, and the generator writes no catalog output or state

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

### Requirement: Support an opt-in location setting

The generator SHALL accept an `include_location` setting resolved from the `--include-location` flag first and the `include_location` key in `media-catalog.toml` second, defaulting to off. The server SHALL accept and ignore the key like other generator-only keys.

#### Scenario: Flag enables location publication
- **WHEN** the generator is started with `--include-location`
- **THEN** published item records include geocoordinates where available

#### Scenario: TOML key enables location publication
- **WHEN** `media-catalog.toml` sets `include_location = true` and no flag is given
- **THEN** published item records include geocoordinates where available

#### Scenario: Flag wins over the file
- **WHEN** the configuration file sets `include_location = false` and `--include-location` is given
- **THEN** location publication is enabled

#### Scenario: Default is off
- **WHEN** neither the flag nor the configuration file sets location publication
- **THEN** no geocoordinates are published

#### Scenario: Server ignores the key
- **WHEN** `media-catalog.toml` contains `include_location = true`
- **THEN** `media-catalog-server` starts normally and ignores the key

#### Scenario: Invalid value rejected
- **WHEN** the configuration file supplies a non-boolean value for `include_location`
- **THEN** the generator exits non-zero with an error naming `include_location` and writes no catalog output or state

### Requirement: Support an opt-in location preview setting

The generator SHALL accept a `location_preview` setting resolved from the `--location-preview` flag first and the `location_preview` key in `media-catalog.toml` second, defaulting to off. The server SHALL accept and ignore the key like other generator-only keys.

#### Scenario: Flag enables the location preview
- **WHEN** the generator is started with `--location-preview`
- **THEN** published pages and JSON advertise the location preview as enabled

#### Scenario: TOML key enables the location preview
- **WHEN** `media-catalog.toml` sets `location_preview = true` with no flag given
- **THEN** published pages and JSON advertise the location preview as enabled

#### Scenario: Flag wins over the file
- **WHEN** the configuration file sets `location_preview = false` and `--location-preview` is given
- **THEN** the location preview is enabled

#### Scenario: Default is off
- **WHEN** neither the flag nor the configuration file sets the location preview
- **THEN** published output advertises no location preview and the viewer makes no third-party map request

#### Scenario: Server ignores the key
- **WHEN** `media-catalog.toml` contains `location_preview = true`
- **THEN** `media-catalog-server` starts normally and ignores the key

#### Scenario: Invalid value rejected
- **WHEN** the configuration file supplies a non-boolean value for `location_preview`
- **THEN** the generator exits non-zero with an error naming `location_preview` and writes no catalog output or state

### Requirement: Reject a location preview without published location

The generator SHALL reject `location_preview` when location publication is disabled, because there are no coordinates to preview. The error SHALL name both settings, and the run SHALL write no catalog output or state.

#### Scenario: Preview enabled while location is off
- **WHEN** `location_preview` resolves to enabled and `include_location` resolves to disabled
- **THEN** the generator exits non-zero with an error naming `location_preview` and `include_location`

#### Scenario: Preview enabled with location published
- **WHEN** both `location_preview` and `include_location` resolve to enabled
- **THEN** validation passes and the run proceeds

#### Scenario: Location enabled without preview
- **WHEN** `include_location` resolves to enabled and `location_preview` is not set
- **THEN** validation passes and only location publication is enabled
