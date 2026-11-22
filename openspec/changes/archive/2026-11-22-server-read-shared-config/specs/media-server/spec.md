# Spec Delta

## MODIFIED Requirements

### Requirement: Control binding and access

The service SHALL resolve its bind address from `--bind`, then `bind` in the shared configuration file, and otherwise default to a loopback address. Access control SHALL be enabled only by a non-empty `MEDIA_CATALOG_TOKEN` environment variable.

#### Scenario: Default binding
- **WHEN** the service starts without a bind address in either flags or the configuration file
- **THEN** it listens only on a loopback address

#### Scenario: Configured binding
- **WHEN** the configuration file declares `bind = "0.0.0.0:8080"` and no `--bind` flag is given
- **THEN** the service listens on that address

#### Scenario: Command-line binding overrides configuration
- **WHEN** the configuration file declares a bind address and `--bind` is also given
- **THEN** the service listens on the flag address

#### Scenario: Missing credential
- **WHEN** access control is enabled and a request omits a valid credential
- **THEN** the service responds with `401`

#### Scenario: Valid credential
- **WHEN** access control is enabled and a request presents a valid credential
- **THEN** the request is served normally

#### Scenario: Token only from the environment
- **WHEN** `MEDIA_CATALOG_TOKEN` is unset or empty
- **THEN** access control is disabled regardless of the configuration file contents

## ADDED Requirements

### Requirement: Limit concurrent media streams

The service SHALL cap simultaneous `/media/` streams at `max_streams`, resolved from `--max-streams`, then `max_streams` in the shared configuration file, otherwise 8, and SHALL reject values below 1.

#### Scenario: Default stream limit
- **WHEN** neither source configures `max_streams`
- **THEN** the service admits up to eight concurrent media streams

#### Scenario: Configured stream limit
- **WHEN** the configuration file declares `max_streams = 2`
- **THEN** the service admits at most two concurrent media streams

#### Scenario: Command-line stream limit overrides configuration
- **WHEN** the configuration file declares `max_streams` and `--max-streams` is also given
- **THEN** the flag value is used

#### Scenario: Stream limit reached
- **WHEN** a media request arrives while the stream limit is already reached
- **THEN** the service responds with `503` and does not open the file

#### Scenario: Zero stream limit rejected
- **WHEN** `max_streams` resolves to `0`
- **THEN** startup fails with a configuration error before binding the listener
