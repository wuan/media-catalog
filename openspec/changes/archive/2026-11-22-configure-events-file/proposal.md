# Proposal

## Why

The events content file can only be located through the `--events` flag or a
silent `./events.toml` probe against the current directory. Every other
deployment input already lives in `media-catalog.toml`, so operators must
remember a second flag, and the implicit probe makes "no events" depend on
where the binary happens to run.

## What Changes

- Add an `events_file` path key to `media-catalog.toml`, alongside the storage
  roots and URL prefixes.
- Keep `--events` as a CLI override, following the existing rule that CLI flags
  override TOML values.
- **BREAKING**: Remove the implicit `./events.toml` probe. When neither
  `events_file` nor `--events` is set, the build has no events; an explicitly
  configured path must exist and be a file.
- Resolve relative events paths against the current directory, matching how
  `media_root`, `data_root`, and `state_root` are resolved.
- Update the README configuration table, example, and Events section, including
  a migration note for setups that relied on the implicit default.
- Update the `event-browsing` spec to describe path configuration and
  precedence.

The events file remains authored content, separate from deployment settings;
only the path that points at it moves into the main configuration. Published
JSON and server behavior are unchanged.

## Capabilities

### New Capabilities

_None._

### Modified Capabilities

- `event-browsing`: The generator locates the authored events file through
  `media-catalog.toml`, with `--events` as an override and no implicit
  `./events.toml` default.

## Impact

- `src/config.rs`: new `FileConfig.events_file`, revised resolution and tests.
- `README.md`: configuration table, TOML example, Events section, and migration
  note.
- `openspec/specs/event-browsing/spec.md`: delta for events-file configuration.
- Tests: configuration precedence, missing-file errors, binary help, and an
  integration path that loads events from the main config.
- Compatibility: setups that only placed `events.toml` beside the binary must
  now declare `events_file`; catalogs without events remain byte-identical.
