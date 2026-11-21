# Tasks

## 1. Configuration and CLI roots

- [x] 1.1 Replace `Cli.catalog_root` with `--data-root` and `--state-root`, replace `FileConfig.catalog_root` with `data_root`/`state_root`, and update `Config` to derive `state_db = state_root/state.sqlite`; verify `cargo check` succeeds and `--help` lists `--data-root` and `--state-root` without `--catalog-root`.
- [x] 1.2 Update `Config::validate` to reject equal or nested roots for every pair, keeping data outside media and state outside data and media; verify new config tests cover equal roots, state-inside-data, data-inside-media, and a valid sibling layout.
- [x] 1.3 Update existing configuration tests and TOML fixtures from `catalog_root` to `data_root`/`state_root`, including an unknown legacy key test; verify `cargo test config` passes and old keys produce a clear parse error.
- [x] 1.4 Update the generator progress line to log the data root and state root instead of the catalog root; verify a fixture run prints both roots and no `catalog root` wording.

## 2. Generator publication and state paths

- [x] 2.1 Rename generator use of `public_root` to `data_root` in `src/lib.rs`, `src/json.rs`, `src/prune.rs`, and `src/paths.rs`, publishing JSON and thumbnails directly under the data root; verify pipeline tests still find `catalog/index.json`, year shards, and thumbnails under the data root.
- [x] 2.2 Update `tests/common/mod.rs` to expose separate `data` and `state` directories, with `state` containing `state.sqlite` and `build.lock`; verify the fixture reports no files outside those trees after a run.
- [x] 2.3 Update `tests/binary.rs` and `tests/pipeline.rs` arguments and assertions to `--data-root`/`--state-root`; verify the binary and pipeline test suites pass, including the negative run that writes no data tree.
- [x] 2.4 Update README generator sections (quick start, catalog-root description, output tree, configuration table, TOML example) to the three-root layout; verify the documented generator command runs against a fixture.

## 3. Server data root

- [x] 3.1 Replace `ServeCli.catalog_root` with `--data-root` and construct `ServerConfig` with the data root directly, renaming `public_root` to `data_root`; verify the server CLI test parses `--data-root` and rejects a missing data root.
- [x] 3.2 Rename `public_root` to `data_root` in `CatalogStore`, catalog snapshot loading, static-file resolution, and server startup logging while keeping `/catalog/*` and `/thumbs/*` routes unchanged; verify server tests for catalog JSON, thumbnails, and rendered pages pass.
- [x] 3.3 Update `tests/server.rs` and `tests/end_to_end.rs` fixtures to build `<data-root>/catalog` and `<data-root>/thumbs` without `public/`; verify all server and end-to-end tests pass.
- [x] 3.4 Add coverage that the server starts when the data root has no catalog, serves the shell, and does not read a legacy `public/catalog/index.json`; verify the startup and legacy-tree tests pass.
- [x] 3.5 Update `docs/server.md`, `docs/deploy/media-catalog-server.service`, `docs/deploy/com.example.media-catalog-server.plist`, and `docs/performance.md` to use `--data-root`; verify no deployment or performance example passes `--catalog-root`.

## 4. Migration and integration

- [x] 4.1 Replace the README upgrade/migration section with the `public/` → data root and `state.sqlite`/`build.lock` → state root move commands, and document that no legacy fallback exists; verify the commands work on a fixture catalog tree.
- [x] 4.2 Remove remaining non-migration references to `catalog-root` and the served `public/` tree from source comments, README, and docs; verify `rg 'catalog-root|public/' src README.md docs` reports only intentional migration text.
- [x] 4.3 Run `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`, and `openspec validate split-catalog-and-state-roots --strict`; verify all pass.

## Workflow follow-up

- Archive and sync `server-render-catalog` before archiving this change, so the modified `static-catalog` and `media-server` requirements apply to the post-render spec state.
- Archive this change after review and verify the resulting main specs describe the three-root layout.
