# Proposal

## Why

The generator currently writes a deployable static HTML tree that the Rust server merely serves. That couples the browser UI to build completion: the catalog has no usable pages until the generator finishes, shell assets are duplicated into `public/`, and stale pages must be cleaned up on disk. Making the Rust server the single runtime and rendering catalog pages dynamically from published JSON lets the catalog become usable as soon as JSON exists, including while thumbnails are still being generated.

## What Changes

- **BREAKING**: The Rust server becomes the only supported runtime. Remove the nginx and rsync/snapshot static deployment paths from documentation.
- The generator stops emitting `index.html`, year/month pages, `search.html`, and asset copies. Its web output becomes JSON shards, thumbnails, and state only.
- The server embeds the application shell, search page, CSS, and JavaScript, and serves them directly from the binary.
- The server renders `/`, `/YYYY/`, and `/YYYY/MM/` dynamically from the published JSON catalog instead of serving pre-generated HTML.
- The server loads an immutable catalog snapshot from `catalog/index.json` plus its year shards, reloads when the index changes, and keeps the previous snapshot if a reload sees a partial publication.
- JSON publication treats `catalog/index.json` as the commit point: shards are written first, then the index.
- Search remains client-side over the JSON index and shards; no server-side search endpoint is added.
- The server can start before any catalog exists. The shell remains available while `/healthz` and catalog routes report the missing catalog.
- Dynamic catalog pages use revalidation-friendly caching with an ETag derived from the snapshot and route; versioned thumbnails remain immutable.
- No TypeScript toolchain is introduced. Browser code stays plain JavaScript with JSDoc types unless the frontend later grows substantially.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `static-catalog`: The catalog web hierarchy is no longer written as deployable static HTML. It is rendered by the Rust runtime from JSON, and the search shell/assets are served by the runtime. JSON output, URL encoding, deterministic catalog data, and no-JS browsing remain requirements.
- `media-server`: The service now owns the shell/assets and dynamically renders catalog pages from JSON, including snapshot loading, revalidation, and missing-catalog behavior. Static file serving is limited to generated JSON, thumbnails, and original media.
- `incremental-builds`: Output atomicity and deletion behavior apply to JSON, thumbnails, and state rather than generated HTML pages. Dynamic pages reflect published JSON without a separate page-regeneration or stale-page cleanup phase.

## Impact

- **Generator pipeline**: remove HTML rendering and asset writes; reorder JSON publication so shards precede the index commit; adjust the run summary away from page counts.
- **Rendering**: reuse the existing Askama templates and catalog model as pure render functions callable by the server.
- **Server**: add catalog snapshot loading/reloading, dynamic page routes, embedded shell/asset routes, ETag handling, and startup without a catalog root.
- **Output tree**: `public/` becomes runtime catalog data rather than a deployable site; the name may be retained for compatibility in this change.
- **Tests**: cover dynamic page rendering, snapshot reload and rollback, missing catalog startup, client-side search over JSON, and absence of generated HTML/assets.
- **Documentation**: replace nginx/snapshot deployment guidance with Rust server deployment and document the catalog build/publish flow.
