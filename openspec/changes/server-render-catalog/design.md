# Design

## Context

See `proposal.md` for motivation. The relevant current constraints are:

- `src/render.rs` writes `index.html`, year/month pages, `search.html`, and asset copies into `public/`.
- `src/server/static_files.rs` serves files from `public/` and currently expects `index.html` for `/`.
- `src/json.rs` writes `catalog/index.json` before the year shards.
- The server validates and canonicalizes `public_root` at startup, so it cannot start before the catalog exists.
- The search UI is plain JavaScript in `assets/search.js` and already relies on the JSON index and shards.

## Goals / Non-Goals

**Goals:**

- Make the Rust server the only runtime and presentation layer.
- Render catalog pages dynamically from published JSON.
- Keep the catalog usable while thumbnails are still being generated.
- Preserve no-JS browsing, client-side search, URL encoding, and deterministic catalog data.
- Avoid a runtime database dependency and avoid introducing a frontend build toolchain.

**Non-Goals:**

- Server-side search or pagination endpoints.
- Changing thumbnail generation or media streaming behavior.
- Renaming `public/` in this change.
- Introducing TypeScript or a bundler.

## Decisions

### Use JSON as the runtime catalog source

The server SHALL build its catalog view from `catalog/index.json` and the referenced `catalog/YYYY.json` shards, not from SQLite.

Why:

- The client-side search already needs the JSON.
- It keeps the server independent of the state schema and SQLite/WAL behavior.
- JSON contains everything needed to render pages and search results.
- The generator can publish JSON before thumbnails finish, so the catalog becomes usable early.

Alternative rejected: reading SQLite at runtime. It would couple the server to the state schema, add a second read model, and provide no benefit over JSON.

### Render catalog pages from an immutable snapshot

The server keeps an immutable `Catalog` snapshot in memory. On each catalog request it checks the metadata of `catalog/index.json`; if the index is unchanged, it uses the current snapshot. If it changed, it loads the index and every referenced shard, builds a new snapshot, and swaps it in only after the complete load succeeds.

If a reload fails because a shard is missing or unreadable, the server keeps the previous snapshot. This prevents a reader from seeing a partially published catalog.

Alternative considered: lazy-loading only the requested shard. That reduces initial memory and parse time, but can mix generations across requests during a publication. The full-snapshot approach is simpler and matches the “catalog is one consistent object” model. If catalog size becomes a problem, a versioned lazy shard cache is a later optimization.

### Reuse Askama templates as pure render functions

The existing templates and `model::Catalog`/`MonthView`/`ItemView` structures remain the rendering model. Refactor `render.rs` so the page functions return HTML strings instead of writing files:

```text
render_index(catalog) -> String
render_year(catalog, year) -> String
render_month(catalog, year, month) -> String
render_search_shell() -> String
```

The server calls these functions from its routes. The generator no longer calls them.

The JSON layer needs a public representation that the server can deserialize. The current private `IndexFile`, `IndexYear`, `YearShard`, and `ShardItem` types in `json.rs` can either be made public or moved into a shared catalog JSON module. The server reconstructs `CatalogItemInput`/`Catalog` values from the shard records.

### Embed shell assets in the binary

The server serves `search.html` and `assets/site.css`/`assets/search.js` from embedded content, not from `public/`.

Use the existing `assets/` files and Askama templates as the shared source. The search shell can be rendered by Askama at startup or on request; the CSS and JS are included with `include_str!` or `include_bytes!`.

Embedded responses need the equivalent of the static handler’s behavior:

- correct content type
- `HEAD` support
- ETag/conditional revalidation
- no directory listing

A content hash computed at startup is sufficient for asset ETags. Rendered catalog pages use an ETag derived from the catalog snapshot identity and route.

### Publish JSON with the index as the commit point

Change JSON publication to:

1. Write new/updated year shards atomically.
2. Replace `catalog/index.json` atomically last.
3. Remove stale shards for years no longer referenced by the new index.

The server observes the new index only after every referenced shard exists. If the generator is interrupted before the index write, the previous index and previous shards remain valid, and the server keeps serving the previous snapshot.

### Keep search client-side

`/search.html` remains a shell that loads `/catalog/index.json` and shards in the browser and filters locally. No server-side query endpoint is added.

The server still needs to serve the JSON files, but they are ordinary generated files under `/catalog/` with revalidation caching.

### Render without JavaScript

Catalog pages remain server-rendered HTML. `/`, `/YYYY/`, and `/YYYY/MM/` work without JavaScript, and month grids keep lazy thumbnail loading and direct `/media/` links.

The search page retains its `<noscript>` fallback and links back to the browsable hierarchy.

### Allow startup without a catalog

`ServerConfig::validate` should no longer fail when `public_root` or catalog JSON is absent. The media root remains a required startup input because `/media/` depends on it.

Behavior without a catalog:

- `/`, `/YYYY/`, `/YYYY/MM/`: `/` returns an embedded bootstrap page explaining that the catalog is not yet available; year/month routes return `503` or `404` consistent with the spec.
- `/search.html` and `/assets/*`: available.
- `/catalog/*`: `404` or `503`; the client search page shows a clear failure state.
- `/healthz`: `503` with catalog-unavailable detail.
- `/media/*`: unchanged, subject to media-root availability.

When the generator publishes JSON, the server begins serving catalog pages without a restart.

### Keep route parsing and security explicit

Dynamic routes only accept numeric year and month segments. They do not resolve HTML from the filesystem. The existing safe path resolution for `/catalog/*`, `/thumbs/*`, and `/media/*` remains in place, and dynamic routes are registered before the static fallback.

Auth middleware applies to embedded shell, dynamic pages, JSON, thumbnails, and media consistently.

### Do not adopt TypeScript

The browser code remains plain JavaScript with JSDoc types where useful. The Rust server owns rendering and routing, so TypeScript would only add a frontend build pipeline and a second toolchain without improving the server.

## Risks / Trade-offs

- [Snapshot reload latency after a large catalog update] → Stat the index on each request and reload only when it changes; keep the previous snapshot during reload. If needed, add rendered-page or shard caches later.
- [Memory use for a large catalog snapshot] → The catalog is metadata plus URLs, not media bytes; a full snapshot is acceptable for the documented archive sizes. Revisit lazy shard loading if memory becomes a constraint.
- [Mixed generations during concurrent publication] → Write shards before the index and load a new snapshot only after the index changes; keep the old snapshot on any incomplete load.
- [Stale HTML files left in `public/` after upgrade] → The server no longer serves them, but they may remain on disk. Document an optional cleanup step; do not automatically delete files outside the thumbnail tree as part of this change.
- [Dynamic rendering per request] → Askama rendering of a month page is cheap relative to thumbnail decoding. Add an in-memory rendered-page cache only if profiling shows a need.
- [Missing catalog startup changes health semantics] → Keep media readiness and catalog readiness separate so an operator can distinguish a missing mount from a missing build.
- [ETag churn when the index is rewritten without content changes] → Acceptable; it only causes a revalidation response, not stale content. A content hash can be added later if churn becomes noisy.

## Migration Plan

1. Refactor rendering into pure functions and make catalog JSON types available to the server.
2. Add the catalog snapshot loader and dynamic routes to the server.
3. Embed the search shell and assets; move `/assets/*` and `/search.html` away from the static file handler.
4. Remove HTML/asset writing from the generator pipeline and adjust the run summary.
5. Change JSON publication to write shards first and the index last.
6. Update tests for dynamic rendering, snapshot reload/rollback, missing-catalog startup, and unchanged client-side search.
7. Update README and deployment docs: Rust server is the only supported runtime; nginx/rsync static deployment is removed.

Rollback is straightforward: keep the previous binary and generator version. The catalog JSON/thumbnail tree format remains compatible; generated HTML files are simply ignored by the new server and can be removed manually.

## Open Questions

- Whether to rename `public/` to a clearer name such as `catalog-data/` in a follow-up change.
- Whether rendered-page caching is needed after measuring real archives.
- Whether a one-time generator cleanup of obsolete HTML/assets is desirable, versus documenting manual cleanup.
