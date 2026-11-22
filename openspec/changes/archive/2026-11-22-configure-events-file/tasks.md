# Tasks

## 1. Configuration resolution

- [x] 1.1 Add `events_file` to `FileConfig` and resolve `--events` over the TOML value over no events; verify `cargo test config` covers CLI precedence, config-only, and neither-set cases.
- [x] 1.2 Remove the implicit `./events.toml` probe from events-path resolution; verify tests show an absent path yields no events while a configured missing path fails with `events file not found`.
- [x] 1.3 Keep configured paths resolved against the current directory and reject non-file paths; verify config tests cover a relative path and a directory path.

## 2. Documentation and CLI surface

- [x] 2.1 Update the README configuration table, TOML example, Events section, and migration note; verify the documented config example loads an events file.
- [x] 2.2 Update binary and integration tests for the `--events` help text and for loading events through `media-catalog.toml`; verify `cargo test --test binary` and the config tests pass.

## 3. Integration check

- [x] 3.1 Run `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test`; verify all pass.
- [x] 3.2 Run `openspec validate configure-events-file --strict`; verify the change validates without errors.

## Workflow follow-up

- Archive the change after review and sync the `event-browsing` delta into the main spec.
- Verify the archived spec reflects the configured-path behavior.
