# Spec Delta

## MODIFIED Requirements

### Requirement: Configure three independent storage roots

The generator SHALL resolve `media_root`, `data_root`, and `state_root` from flags or the shared `media-catalog.toml`. The runtime server SHALL resolve only `media_root` and `data_root` and SHALL NOT require or read a state root.

#### Scenario: Generator start
- **WHEN** the generator starts with all three roots resolved
- **THEN** it reads originals from the media root, publishes data under the data root, and stores build state under the state root

#### Scenario: Generator start from configuration
- **WHEN** the generator starts with `media-catalog.toml` declaring all three roots and no root flags
- **THEN** it uses the configured roots without requiring the flags

#### Scenario: Server start
- **WHEN** the server starts with media and data roots resolved
- **THEN** it serves published data and originals without requiring or reading a state root

#### Scenario: Server roots from configuration
- **WHEN** `media-catalog.toml` declares `media_root` and `data_root` and `media-catalog-server` starts without root flags
- **THEN** it uses the configured media and data roots

#### Scenario: Server flags override configured roots
- **WHEN** the server is started with `--data-root` or `--media-root` while the configuration file declares those roots differently
- **THEN** the flag paths are used for the corresponding roots

#### Scenario: Missing data root
- **WHEN** either binary is started without `--data-root` and without `data_root` in the configuration file
- **THEN** it exits with a clear configuration error instead of deriving a data location

### Requirement: Assign root access by role

The generator SHALL read the media root and write the data and state roots. The server SHALL read the data and media roots only and SHALL NOT open the state root.

#### Scenario: Generator permissions
- **WHEN** the generator runs
- **THEN** it needs read access to media and write access to data and state, and it never modifies originals

#### Scenario: Server permissions
- **WHEN** the server runs
- **THEN** it needs read-only access to data and media and no access to state

#### Scenario: Configured state root ignored by the server
- **WHEN** the shared configuration file declares a state root and the server starts with it
- **THEN** the server serves normally without opening, validating, or requiring that state root
