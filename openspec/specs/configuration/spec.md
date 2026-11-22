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
