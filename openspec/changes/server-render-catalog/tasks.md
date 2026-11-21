# Tasks

## 1. Shared Catalog Model and Rendering

- [x] 1.1 Make the JSON catalog types available outside the generator and parse index plus shards into the shared `Catalog` model; verify with a round-trip unit test using a fixture index and shard.
- [x] 1.2 Refactor the renderer into pure functions such as `render_index`, `render_year`, `render_month`, and `render_search_shell`; verify snapshot or string tests cover years, months, item grids, placeholders, and escaping.
- [x] 1.3 Remove file-writing and stale-page cleanup from the rendering module while keeping the templates reusable by the server; verify the generator no longer depends on a render-to-disk path.

## 2. Server Catalog Snapshot

- [x] 2.1 Implement catalog snapshot loading from `catalog/index.json` and referenced shards, including index metadata checks and immutable snapshot replacement; verify unit tests cover initial load, index change, and missing shard handling.
- [x] 2.2 Ensure an incomplete reload keeps the previous snapshot and does not expose a partial catalog; verify a test where one referenced shard is missing.
- [x] 2.3 Add dynamic routes for `/`, `/{year}/`, and `/{year}/{month}/` that render from the snapshot and return `404` for unknown years or months; verify router tests for each route.
- [x] 2.4 Add revalidation behavior for rendered pages using an ETag derived from the snapshot and route; verify a matching `If-None-Match` returns `304`.
- [x] 2.5 Register dynamic routes before the static fallback and accept only numeric year/month segments; verify traversal and invalid-segment tests still pass.

## 3. Embedded Shell and Assets

- [x] 3.1 Serve `/search.html`, `/assets/site.css`, and `/assets/search.js` from embedded content with correct content types, `HEAD`, and validators; verify server tests do not depend on files in `public/assets`.
- [x] 3.2 Add an embedded root bootstrap page for requests to `/` when no catalog is published; verify the page states that the catalog is unavailable and does not return an empty response.
- [x] 3.3 Update the client search shell to show a clear catalog-unavailable message when JSON loading fails, while retaining client-side filtering; verify the Node filter test and a missing-catalog test.
- [x] 3.4 Allow the server to start without `public/` or catalog JSON while still requiring the media root; verify `/healthz`, shell routes, and catalog routes behave according to the spec.

## 4. Generator Output and JSON Publication

- [x] 4.1 Remove HTML page and asset writes from the generator pipeline and drop the page-render phase; verify pipeline tests assert JSON and thumbnails exist while generated HTML/assets do not.
- [x] 4.2 Adjust the run summary and report to remove or redefine page counts while keeping JSON, thumbnail, deletion, and prune counts; verify report tests and CLI output.
- [x] 4.3 Reorder JSON publication so all year shards are written before `catalog/index.json`, then remove stale shards; verify tests cover publication order and a missing-shard failure path.
- [x] 4.4 Keep deletion and prune behavior confined to JSON/state and `public/thumbs/`; verify deleted items disappear from JSON and orphan thumbnails are only removed with `--prune`.

## 5. Integration

- [x] 5.1 Add an end-to-end fixture test that runs the generator and then the server, asserting dynamic index/year/month pages, JSON, thumbnails, originals, and client-side search data.
- [x] 5.2 Verify the running server picks up a catalog update without restart after a second generator run changes an item; assert the rendered page and ETag change.
- [x] 5.3 Verify no-JS catalog pages still render, escape hostile filenames, and link to `/media/` originals.
- [x] 5.4 Verify existing media streaming, byte-range, auth, and graceful-shutdown behavior is unchanged by the route changes.

## 6. Documentation and Deployment

- [x] 6.1 Rewrite the README architecture and quick-start sections for a server-only runtime: generator publishes JSON/thumbnails, server renders pages and serves media.
- [x] 6.2 Remove nginx and rsync/static-snapshot deployment as supported paths and document systemd/launchd operation and catalog snapshot reload behavior.
- [x] 6.3 Update `docs/server.md` route tables, caching behavior, missing-catalog startup, and embedded shell/asset handling.
- [x] 6.4 Document the migration step for obsolete `index.html`, year/month HTML, `search.html`, and `assets/` left in `public/` after upgrade.
- [x] 6.5 Run `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`, and the Node search test; verify all pass.

## Workflow follow-up

- Run `openspec validate server-render-catalog --strict` before archive.
- Archive the change after review and implementation are complete.
