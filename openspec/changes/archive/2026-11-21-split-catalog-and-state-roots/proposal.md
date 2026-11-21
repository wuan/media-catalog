# Proposal

## Why

`--catalog-root` currently owns two unrelated concerns: the published catalog
data served at runtime (`catalog/*.json` and `thumbs/`) and the generator's
SQLite build state (`state.sqlite`, `build.lock`). The `public/` wrapper exists
only to keep build state outside the served tree, yet the name is a leftover
from static-site deployment and no longer describes the content. The server must
also accept a catalog root it never writes to and derive `public/` from it.

Separate roots make ownership explicit: originals are read-only input, published
data is the served output and backup target, and state is a disposable local
cache. This also removes a misleading directory name before more deployments
depend on it.

## What Changes

- **BREAKING** Replace `--catalog-root` with two independent flags:
  `--data-root` for published catalog data and `--state-root` for build state.
- **BREAKING** Remove the `public/` wrapper. The data root is the served tree
  itself:
  - `<data-root>/catalog/index.json` (commit point)
  - `<data-root>/catalog/YYYY.json`
  - `<data-root>/thumbs/...`
- Store build state directly in the state root:
  - `<state-root>/state.sqlite`
  - `<state-root>/build.lock`
- Generator CLI becomes `--media-root`, `--data-root`, `--state-root`.
- Server CLI becomes `--media-root`, `--data-root`; it never reads the state
  root.
- Validate that the three roots are pairwise separate, that the state root is
  not inside the data or media root, and that the data root is not inside the
  media root.
- **BREAKING** Drop support for the legacy `<catalog-root>/public` layout. There
  is no compatibility fallback; existing deployments move the directories.
- Update configuration examples, deployment units, migration documentation, and
  tests to the new layout.

Out of scope: changing catalog URL routes, changing the JSON schema, moving
media, or renaming thumbnail URLs.

## Capabilities

### New Capabilities

- `storage-roots`: Defines the three independently configured storage roots,
  their contents, separation rules, and which binaries read or write each root.

### Modified Capabilities

- `static-catalog`: Published JSON is emitted directly under the configured data
  root instead of a `public/` subtree.
- `incremental-builds`: SQLite state and the build lock live under the
  configured state root, separate from published data.
- `thumbnail-generation`: Thumbnails are written under `<data-root>/thumbs/`
  rather than `public/thumbs/`.
- `media-server`: The service reads published data directly from `--data-root`
  and has no access to the state root.

## Impact

- `src/config.rs`: replace `catalog_root`, `public_root`, and derived `state_db`
  with `data_root` and `state_root`; adjust validation and tests.
- `src/server/mod.rs` and related server modules: accept `--data-root` and serve
  it directly; rename `public_root` terminology.
- `src/lib.rs`, `src/json.rs`, `src/prune.rs`, `src/paths.rs`, `src/lock.rs`:
  publish, prune, and lock paths under the new roots.
- `README.md`, `docs/server.md`, `docs/deploy/*`, and configuration examples.
- CLI integration tests and configuration tests.
