# Proposal

## Why

The generator and the server are deployed together against the same media and
data roots, but only the generator reads `media-catalog.toml`. Operators must
repeat the storage paths in `media-catalog-server` flags, and cannot keep bind
address or stream limits with the rest of the deployment configuration. One
shared file reduces drift between the two binaries and makes service definitions
shorter.

## What Changes

- `media-catalog-server` SHALL read the shared `media-catalog.toml`, discovered
  as `./media-catalog.toml` when `--config` is omitted, exactly like the
  generator.
- `--data-root`, `--media-root`, `--bind`, and `--max-streams` become optional
  overrides. Each value resolves as CLI flag, then TOML key, then built-in
  default where a default exists.
- `media-catalog.toml` gains optional server keys: `bind` (socket address) and
  `max_streams` (positive integer). The generator accepts them in the shared
  file but ignores them.
- Missing storage roots now fail during configuration resolution with an error
  that names both the flag and the TOML key, instead of failing during clap
  argument parsing.
- The server continues to read the access token from `MEDIA_CATALOG_TOKEN` only;
  the token is not a configuration-file key.
- **BREAKING** for embedders of the library: `ServeCli::into_config` becomes
  fallible because it now loads and resolves the TOML file. The binary handles
  the error as a configuration failure and exits non-zero.

## Capabilities

### New Capabilities

- `configuration`: shared `media-catalog.toml` discovery, key set, source
  precedence, relative-path resolution, and validation.

### Modified Capabilities

- `storage-roots`: roots may come from the shared file as well as flags, with
  flags overriding the file, and the server still requires only media and data
  roots.
- `media-server`: bind address and stream limit may be configured in the shared
  file with CLI overrides; startup reports configuration errors clearly.

## Impact

- `src/config.rs`: shared TOML struct gains server keys; file loading becomes
  reusable outside the generator.
- `src/server/mod.rs`: `ServeCli`, `ServerConfig` resolution, validation, and
  tests change.
- `src/bin/media-catalog-server.rs`: configuration resolution may fail before
  serving.
- `README.md` and `docs/server.md`: document shared file usage and server keys.
- `docs/deploy/*` examples may be simplified but remain valid with explicit
  flags.
- Integration tests for server CLI precedence and configuration errors.
