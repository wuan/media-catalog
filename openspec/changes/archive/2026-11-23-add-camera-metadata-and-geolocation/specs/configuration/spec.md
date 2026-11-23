# Spec Delta

## ADDED Requirements

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
