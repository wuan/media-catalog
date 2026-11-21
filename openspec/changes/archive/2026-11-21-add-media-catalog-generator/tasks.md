# Tasks

## 1. Project scaffolding

- [x] 1.1 Initialize the Cargo package with `src/lib.rs`, `src/main.rs`, and the module files described in `design.md`; verify `cargo check` passes.
- [x] 1.2 Add runtime and dev dependencies (`clap`, `serde`, `serde_json`, `toml`, `rusqlite` with bundled SQLite, `image`, `walkdir`, `rayon`, `anyhow`, `thiserror`, `askama`, `percent-encoding`, `indicatif`, `fs2`, and `insta`); verify `cargo test` compiles with placeholder tests and `cargo build` succeeds.

## 2. Configuration and build state foundation

- [x] 2.1 Implement CLI flags plus optional TOML configuration with the documented defaults and precedence; verify unit tests cover CLI-over-TOML precedence, missing required roots, prefix normalization, and rejection of a state path inside `public/`.
- [x] 2.2 Validate the media root and check that `ffmpeg`/`ffprobe` are available before scanning; verify a test with a restricted `PATH` produces a clear configuration error and nonzero exit.
- [x] 2.3 Implement the SQLite schema, schema version, open/create logic, and per-item upsert helpers; verify an idempotent-init test creates the tables and preserves existing rows on a second open.
- [x] 2.4 Implement the build lock beside the state database; verify a test run cannot acquire the lock while another holder exists and that the lock is released after a failed run.
- [x] 2.5 Document configuration flags, defaults, state location, and the ffmpeg requirement in the README; verify each documented command runs as written against a tiny fixture.

## 3. Scan, classification, and fingerprints

- [x] 3.1 Implement recursive scanning of `YYYY/MM/**`, year/month parsing, image/video classification, hidden and unsupported-file skipping, and per-file error collection; verify unit tests cover grouping, nested month folders, ignored files, and unreadable entries.
- [x] 3.2 Implement the size+mtime fingerprint and URL-safe thumbnail version; verify unit tests show stability for unchanged files and a new version when size or mtime changes.
- [x] 3.3 Build the deterministic catalog model with newest-first years/months and capture-timestamp-then-path item ordering; verify unit tests cover tie-breaks and missing-timestamp fallback.
- [x] 3.4 Add a scan integration fixture under `tests/`; verify it reports the expected item count, month grouping, and error list.
- [x] 3.5 Document supported extensions, folder-authoritative grouping, and the metadata privacy rule; verify the README matches the implemented extension list.

## 4. Metadata probing

- [x] 4.1 Implement image probing for dimensions, EXIF orientation, and capture timestamp with mtime fallback; verify tests cover upright, rotated, tagged, and untagged images.
- [x] 4.2 Implement `ffprobe` JSON parsing for video dimensions, duration, and rotation; verify with a checked-in video fixture or a test that skips cleanly when ffmpeg is unavailable.
- [x] 4.3 Cache probe results and fingerprints in SQLite so unchanged items skip probing; verify an integration test shows a second run does not reopen unchanged image/video sources.
- [x] 4.4 Ensure the serialized catalog model exposes no GPS or camera fields; verify a fixture with geotagged EXIF produces JSON without those keys.
- [x] 4.5 Document extracted metadata fields and the naive-local capture timestamp rule; verify the documented record shape matches a generated fixture.

## 5. Thumbnail generation

- [x] 5.1 Implement the decoder boundary and v1 raster decoder for JPEG, PNG, and WebP; verify supported formats succeed and HEIC is reported as unsupported without aborting.
- [x] 5.2 Implement orientation handling followed by center cover-crop and WebP encoding at 512x512; verify tests assert exact dimensions and upright output for landscape, portrait, and rotated images.
- [x] 5.3 Implement video poster extraction at the representative timestamp with a short-video fallback; verify fixture tests produce a 512x512 poster or a recorded failure without aborting.
- [x] 5.4 Implement path mirroring under `public/thumbs/`, version query URLs, and temp-file-plus-rename writes; verify tests check unique paths, stable versions, and no partial file after a simulated write failure.
- [x] 5.5 Record thumbnail status and retry failed thumbnails on later runs; verify a forced failure is retried and succeeds when the fixture becomes readable.
- [x] 5.6 Document the thumbnail size, format, crop, versioning, video poster behavior, and ffmpeg dependency; verify the README commands generate a thumbnail from a fixture.

## 6. Static HTML rendering

- [x] 6.1 Implement askama templates and embedded assets for the root years page, year months page, and month grid; verify snapshot tests cover empty, single-year, and multi-year catalogs.
- [x] 6.2 Implement root-relative URL builders with configurable prefixes and per-segment percent encoding; verify unit tests cover spaces, Unicode, `#`, `%`, and nested folders.
- [x] 6.3 Render month grids with lazy loading, video markers/durations, breadcrumbs, and missing-thumb placeholders; verify a snapshot test with a ~500-item month stays bounded and every item links to `/media/`.
- [x] 6.4 Verify HTML escaping and accessibility: a filename containing `<script>` is inert in output, and every thumbnail has non-empty alternative text; cover both with snapshot or unit assertions.
- [x] 6.5 Document the generated web tree and page navigation in the README; verify the described paths exist after a fixture build.

## 7. JSON catalog and search page

- [x] 7.1 Emit `catalog/index.json` and one `catalog/YYYY.json` shard per year with the documented item fields and deterministic ordering; verify snapshot tests and byte-identical output for two unchanged runs.
- [x] 7.2 Implement `search.html` and `assets/search.js` to load the index and shards in parallel and filter by text, type, year, and month; verify against a fixture served locally that each filter narrows the grid correctly.
- [x] 7.3 Reflect filters in the URL query and provide a no-JavaScript fallback linking to the browse hierarchy; verify shareable URL restoration and the fallback page.
- [x] 7.4 Document search behavior and shard layout; verify the README's shard examples match generated fixture output.

## 8. Incremental orchestration and cleanup

- [x] 8.1 Orchestrate scan → diff → probe changed → thumbnail changed → render all pages + JSON → state updates; verify an integration test with new, changed, and unchanged items processes only the first two.
- [x] 8.2 Handle interrupted builds by recording thumbnails only after successful writes; verify a simulated mid-run abort resumes and completes without regenerating finished thumbnails.
- [x] 8.3 Reflect deletions in pages/JSON and implement opt-in pruning confined to `public/thumbs/`; verify deleted items disappear, orphan thumbs are removed only with `--prune`, and nothing outside the thumb tree is touched.
- [x] 8.4 Add a mount sanity check for a sharp item-count drop and require an explicit override before destructive pruning; verify the warning and override behavior with a fixture that loses most items.
- [x] 8.5 Implement `--dry-run`, progress reporting, the run summary, and nonzero exit when item errors occur; verify output and exit codes for clean and failing fixtures.
- [x] 8.6 Document incremental behavior, resume, prune safety, and the mount-drop warning; verify documented flags match the CLI help.

## 9. End-to-end verification and release

- [x] 9.1 Run the full integration suite against a fixture tree; verify idempotency, deterministic output, failure tolerance, lock behavior, and that no files are written outside the public tree except state/lock files.
- [x] 9.2 Exercise the binary against a copied subset of the real archive (or a synthetic 75k-item tree); record initial and incremental timings, memory use, and resume behavior, and confirm output paths resolve under `/thumbs/` and `/media/`.
- [x] 9.3 Write deployment documentation for nginx (`/` and `/media/` mounts), rsync/snapshot deployment, caching headers, and the initial long-running build; verify the commands work against a fixture.
- [x] 9.4 Run `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`, and `cargo build --release`; verify all pass and the release binary starts with `--help`.

## Workflow follow-up

- Archive the change after the project's review requirements are satisfied.
- Verify the archived specs and tasks reflect the delivered behavior.
