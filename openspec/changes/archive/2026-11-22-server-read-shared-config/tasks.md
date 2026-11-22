# Tasks

## 1. Shared configuration schema and loader

- [x] 1.1 Expose reusable TOML loading from `src/config.rs` (for example `FileConfig::load`) and make `Config::load` use it; verify `cargo test config::` passes and the existing `events_load_through_media_catalog_toml` pipeline test still passes.
- [x] 1.2 Add optional `bind: Option<String>` and `max_streams: Option<usize>` to `FileConfig`, keeping `deny_unknown_fields`; add unit tests that parse both keys, reject an unknown key, and confirm the generator resolves normally while ignoring the server keys; verify with `cargo test config::`.
- [x] 1.3 Update the README Configuration section so one `media-catalog.toml` schema covers both binaries, lists `bind` and `max_streams`, and states CLI-over-file precedence and working-directory path resolution; verify every documented key appears in the schema test or example.

## 2. Server configuration resolution

- [x] 2.1 Add `--config` to `ServeCli` and change `data_root`, `media_root`, `bind`, and `max_streams` to optional fields; implement flag-then-file-then-default resolution that parses `bind` into a socket address and names both flag and TOML key in errors; verify with unit tests for file-only roots, CLI override, default bind/max-streams, invalid bind, and missing roots.
- [x] 2.2 Make `ServeCli::into_config` fallible and update `src/bin/media-catalog-server.rs` to print the configuration error and exit non-zero before serving; verify `cargo build` and a manual run with `--config /missing.toml` exits non-zero naming the file.
- [x] 2.3 Keep `MEDIA_CATALOG_TOKEN` as the only credential source and ensure a configured `state_root` is never opened by the server; verify tests cover token behavior and startup with a config file that declares `state_root` while serving from media and data roots.
- [x] 2.4 Expand the max-stream error message to mention `--max-streams` and `max_streams`, and confirm validation still runs before the listener binds; verify with the zero-limit unit test and `cargo test server::`.

## 3. Binary, integration, and documentation coverage

- [x] 3.1 Add server CLI tests covering `--help` documentation of `--config`, `--bind`, `--max-streams`, and the missing-root configuration error when no file supplies roots; verify with `cargo test --test binary`.
- [x] 3.2 Add an integration case that writes `bind` and `max_streams` into `media-catalog.toml` and runs the generator from that file, proving the shared file remains valid for generation; verify with `cargo test --test pipeline`.
- [x] 3.3 Update `docs/server.md` to show starting from the shared file, document the resolution order and `--config` for supervisors, and keep deploy examples valid with explicit flags; verify the documented command starts with a sample file and the unit examples still reference existing flags.
- [x] 3.4 Run the full suite (`cargo test`) and confirm no existing generator or server behavior regresses.

## Workflow follow-up

- Archive the change after review and verification are complete.
- Confirm the archived specs update `configuration`, `storage-roots`, and `media-server` as intended.
