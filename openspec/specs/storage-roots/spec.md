# storage-roots Specification

## Purpose

Defines the three independently configured storage roots for originals, published catalog data, and rebuildable build state, including their contents, separation rules, and which binaries access each root.

## Requirements

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

### Requirement: Keep storage roots separate

Configuration validation SHALL reject overlapping roots: no two roots may be equal or nested, the state root SHALL NOT be inside the data or media root, and the data root SHALL NOT be inside the media root.

#### Scenario: Equal roots
- **WHEN** any two configured roots resolve to the same directory
- **THEN** startup fails with an error naming the conflicting roots

#### Scenario: State inside data
- **WHEN** the state root is nested inside the data root
- **THEN** startup fails because build state would be part of the served tree

#### Scenario: Data inside media
- **WHEN** the data root is nested inside the media root
- **THEN** startup fails because published output would be written into the read-only source tree

#### Scenario: Valid layout
- **WHEN** the three roots are distinct sibling directories
- **THEN** validation succeeds

### Requirement: Lay out published data under the data root

The data root SHALL be the published tree itself, containing `catalog/index.json`, `catalog/YYYY.json` shards, and `thumbs/`, with no intermediate `public/` directory.

#### Scenario: Published tree layout
- **WHEN** the generator publishes catalog data with the data root at `./data`
- **THEN** the commit point is `./data/catalog/index.json`, year shards are `./data/catalog/YYYY.json`, and thumbnails are under `./data/thumbs/`

#### Scenario: No public wrapper
- **WHEN** the generator or server resolves published catalog paths
- **THEN** it joins them directly to the data root and does not insert `public/`

### Requirement: Lay out build state under the state root

The state root SHALL contain only rebuildable build state: `state.sqlite` and `build.lock`, stored side by side and outside the published tree.

#### Scenario: State tree layout
- **WHEN** the generator runs with the state root at `./state`
- **THEN** it creates or opens `./state/state.sqlite` and acquires `./state/build.lock`

#### Scenario: State is disposable
- **WHEN** the entire state root is deleted
- **THEN** the published data root remains intact and the next generator run rebuilds state from the media root

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

### Requirement: Reject the legacy catalog-root layout

The binaries SHALL NOT accept `--catalog-root` and SHALL NOT fall back to reading or writing a legacy `public/` subtree.

#### Scenario: Legacy flag
- **WHEN** a command is run with `--catalog-root`
- **THEN** argument parsing fails with an unknown-argument error

#### Scenario: Legacy tree present
- **WHEN** a directory contains only the legacy `public/catalog/index.json` layout and no data root catalog
- **THEN** the server reports the catalog as unavailable instead of reading that legacy subtree
