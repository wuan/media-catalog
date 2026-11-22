# Design

## Context

See `proposal.md` for motivation and the delta specs for behavior.

Current state:

- `src/config.rs` owns the generator's clap `Cli`, the TOML `FileConfig`
  (`deny_unknown_fields`, every field optional), `Config::load/resolve`, and a
  private `load_file_config` that discovers `./media-catalog.toml` or honors
  `--config`.
- `src/server/mod.rs` owns `ServeCli` with required `--data-root` and
  `--media-root` path arguments, `--bind` defaulting to `127.0.0.1:8080`, and
  `--max-streams` defaulting to 8. `ServeCli::into_config` is infallible and
  only reads `MEDIA_CATALOG_TOKEN`.
- `ServerConfig::validate` canonicalizes roots against the working directory,
  requires the media root to exist, allows a missing data root, and rejects
  overlapping roots.
- The server must never open the generator's state root, even if the shared file
  declares `state_root`.
- The access token is deliberately environment-only; the configuration file
  must not become a token store.

## Goals / Non-Goals

**Goals:**

- One parser and one discovery rule for `media-catalog.toml` in both binaries.
- Per-setting precedence: CLI flag, TOML value, built-in default.
- Existing server invocations with explicit flags keep working unchanged.
- Configuration errors are reported before the server binds or the generator
  writes anything.

**Non-Goals:**

- Changing the TOML file format beyond adding optional `bind` and `max_streams`.
- Moving the access token into configuration.
- Changing catalog layout, HTTP behavior, or the generator pipeline.
- Making the server validate or access the state root.
- Loading configuration relative to the config file's own directory.

## Decisions

### Reuse the generator's `FileConfig`

Add `bind: Option<String>` and `max_streams: Option<usize>` to the existing
`FileConfig` and let both binaries parse it. Expose file loading as a reusable
API, for example `FileConfig::load(explicit: Option<&Path>) -> Result<FileConfig>`,
and keep `Config::load` using it.

**Why:** `FileConfig` uses `deny_unknown_fields`. A separate server schema would
either reject generator keys such as `state_root`/`events_file` or require
relaxing typo protection for both binaries. One schema means the shared file can
contain every deployment setting and typos still fail everywhere.

**Alternative considered:** split structs (`CommonConfig`, `GeneratorConfig`,
`ServerConfig`) combined with `#[serde(flatten)]`. More types and error paths for
little benefit; the server has only two extra keys.

### Add optional server settings to the shared schema

`bind` is stored as an optional string and parsed into `SocketAddr` during
resolution so the error can name both `--bind` and `bind`. `max_streams` is an
optional integer validated as at least 1. The generator parses and ignores both
keys.

**Why:** Storing `SocketAddr` directly in `FileConfig` needs a serde adapter or
extra dependency; parsing once at resolution keeps the schema simple and the
error message precise.

**Alternative considered:** custom `deserialize_with` for `SocketAddr`. It moves
the failure into generic TOML parse errors that are harder to attribute to the
`bind` key.

### Give `ServeCli` optional values and a fallible resolution step

`ServeCli` gains `config: Option<PathBuf>`, and `data_root`, `media_root`, `bind`,
and `max_streams` become `Option` fields so clap no longer requires or defaults
them. Introduce a resolution function, such as
`ServerConfig::resolve(cli, file) -> Result<ServerConfig>`, that applies
flag-over-file-over-default precedence and validates the bind string.
`ServeCli::into_config` remains the binary-facing entry point but returns
`Result<ServerConfig>`, loading the file and reading the token environment
variable before delegating.

**Why:** Resolution without clap defaults is what makes the file authoritative
when a flag is absent, and it makes precedence unit-testable without touching the
environment or filesystem. Keeping `into_config` as the entry point minimizes
churn in the binary and embedders.

**Alternative considered:** clap `default_value` for bind/max-streams and then
treating defaults as if they were CLI values. That would make the file values
unreachable, so the defaults must move into resolution.

### Keep filesystem semantics in `ServerConfig::validate`

Resolution resolves paths lexically against the working directory where needed;
`validate` keeps the existing canonicalization, media-root requirement, data-root
optionality, overlap rejection, and max-streams guard. Only the error text for a
bad stream limit expands to name both `--max-streams` and `max_streams`.

**Why:** this preserves the server's startup contract and its distinct handling
of a missing catalog versus a missing media mount. Changing path semantics at the
same time as configuration loading would be an unrelated behavior change.

### Token remains environment-only

`FileConfig` gets no token field. The server continues to read
`MEDIA_CATALOG_TOKEN` in `into_config`.

**Why:** deployment secrets already live in an environment file for systemd and
`launchctl setenv` for macOS; putting a token in TOML would spread secrets into a
file likely read by both binaries and copied between hosts.

### Document explicit `--config` for supervisors

Default discovery stays working-directory based, matching the generator. Service
definitions should pass `--config /path/to/media-catalog.toml` because systemd
and launchd do not guarantee the project directory as the working directory.

## Risks / Trade-offs

- **Generator silently accepts `bind`/`max_streams`** → intentional for one
  schema; document that only the server consumes them. Typos outside the shared
  schema still fail.
- **Working-directory discovery is implicit** → keep the generator's existing
  rule, document `--config` in `docs/server.md`, and keep deploy examples
  explicit.
- **Public library API break** (`into_config` now returns `Result`) → the crate
  is pre-1.0 and in-repo consumers are updated together; note it in the proposal.
- **Two code paths for relative paths** (generator absolutizes lexically, server
  canonicalizes) → both resolve against the working directory today; tests pin
  the shared behavior rather than changing it.
- **Bind parse errors could look like TOML errors** → parse in resolution and
  name the setting in the message.
- **A missing media root still fails startup** → unchanged existing behavior;
  this change only alters where the path comes from.

## Migration Plan

1. Existing invocations with `--data-root`, `--media-root`, `--bind`, and
   `--max-streams` continue to win over any file values, so deployments need no
   immediate change.
2. To adopt the shared file, move roots and server settings into
   `media-catalog.toml`; optionally pass `--config` from the systemd/launchd
   definition.
3. Rollback is simply restoring explicit flags; the file keys are optional.
