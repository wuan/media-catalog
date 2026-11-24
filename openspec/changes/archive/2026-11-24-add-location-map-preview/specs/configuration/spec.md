# Spec Delta

## ADDED Requirements

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
