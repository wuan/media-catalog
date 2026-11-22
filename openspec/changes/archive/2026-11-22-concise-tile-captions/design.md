# Design

## Context

See `proposal.md` for motivation. The current pipeline probes each decodable item, stores metadata in SQLite, publishes yearly JSON shards, and lets the server render month, event, and search tiles from that JSON. Tile captions live in `templates/tiles.html` and `assets/search.js`; the viewer reads tile attributes and keeps its own filename title and full timestamp.

Canonical timestamps are stored as `YYYY-MM-DD HH:MM:SS` and drive ordering and event minute matching. The current privacy boundary strips camera metadata entirely; JSON tests reject `gps`, `camera`, `make`, `model`, latitude, and longitude.

## Goals / Non-Goals

**Goals:**

- Capture a camera model when the source provides one and carry it through state, JSON, and rendering.
- Render `YY-MM-DD HH:MM <model>` as one line on month, event, and search grid tiles, falling back to the timestamp alone.
- Preserve canonical timestamps, deterministic output, and the existing viewer behavior.
- Publish only the model while keeping GPS, location, serial numbers, and other camera details excluded.

**Non-Goals:**

- Redesigning the viewer caption; it keeps the filename title and full timestamp.
- Adding model text to root or year index pages, which contain no item tiles.
- Changing thumbnail metadata or embedding model data in thumbnails.
- Extending text search to match the model (noted as a follow-up).
- Localizing or reformatting timestamps beyond the requested short display form.

## Decisions

### Capture EXIF `Model` and ffprobe model tags

Add `model: Option<String>` to `ExifMetadata` and `ProbeResult`. Read EXIF tag `0x0110` from IFD0 for JPEGs, alongside orientation and capture time. For videos, look up common ffprobe tag names—`com.apple.quicktime.model`, then `model`—in format tags and stream tags.

Trim surrounding whitespace and collapse internal runs of whitespace so the caption cannot wrap on stray spacing. Treat empty values as absent. Alternatives considered: combining `Make` and `Model`, falling back to the filename, or using the media kind. All were rejected because the requested label is the model and missing values should degrade to the timestamp alone.

### Carry the model as an optional field end to end

Add `model` to the state `ItemRecord`, `CatalogItemInput`, `ItemView`, and `ShardItem`. Serialize the shard field only when known (`skip_serializing_if` on `Option<String>`). This keeps old catalogs and items without camera metadata unchanged in shape and keeps determinism.

The model becomes published data, so the JSON privacy test changes from rejecting `make`/`model`/`camera` to rejecting GPS, location, latitude/longitude, and serial fields. The `media-inventory` privacy requirement is updated to allow model while excluding everything else.

### Keep the canonical timestamp and format only at display time

Do not change stored timestamps. Keep `captured_at` as `YYYY-MM-DD HH:MM:SS` for ordering, event membership, JSON consumers, and the viewer. Add a separate short display value for tile captions.

- Rust: `TileView` keeps the raw timestamp for `data-captured` and carries a separate `caption_time` rendered as `%y-%m-%d %H:%M`.
- Search: `assets/search.js` keeps `item.captured_at` in `data-captured` and derives the short caption text with a small helper.

Alternatives considered: changing the stored format (breaks ordering and event matching), or publishing a derived field in JSON (unnecessary schema surface). Both rejected.

### Use a single-line caption with optional model

The `figcaption` becomes one line: a `<time>` element followed by an optional model span, separated by normal whitespace. CSS switches the caption from a column to an inline row with `white-space: nowrap` and ellipsis overflow. The search result builder produces the same text.

The filename leaves the visible caption but remains in the thumbnail `alt` text. The viewer previously read the `.name` span; it falls back to `data-item-path`, which yields the same filename, so the viewer title and original-link label remain meaningful.

### Version both schemas and backfill metadata once

Bump the state schema to 2 and add a `model TEXT` column. Because `NULL` model is also the legitimate result for files without camera metadata, `model IS NULL` cannot drive re-probing without re-probing unchanged files forever. The migration clears cached probe fields (`width`, `height`, `duration_ms`, `captured_at`, `model`) once so unchanged rows are probed again; thumbnails remain valid and are reused.

Bump `JSON_SCHEMA_VERSION` to 2. The server rejects older shards with an explicit rebuild message and keeps serving the previous complete snapshot until a new one lands. Alternatives considered: keeping schema version 1 and treating `model` as an optional extension. Rejected because the field is part of the published contract and the project already validates exact versions.

### Prefer common video model tags and keep lookup narrow

ffprobe exposes model information under varied tag names depending on the recorder. Start with `com.apple.quicktime.model` and `model`; add more keys only when real samples show a need. A missing tag simply yields no model and the date-only caption.

## Risks / Trade-offs

- **Published model metadata** → Only the model is published; GPS, location, serial, and other camera details stay out. Specs and JSON tests enforce the boundary.
- **Model often absent or cryptic** → The date-only fallback is expected and consistent; cryptic EXIF model codes are preferable to inventing a friendlier name.
- **One-time re-probe after migration** → The migration clears cached metadata once. Thumbnails are not regenerated, keeping the cost bounded.
- **Two-digit year is ambiguous** → Display-only `YY`; canonical data and ordering keep the full year.
- **Filename disappears from tiles** → Alternative text retains the filename, and the viewer falls back to `data-item-path`.
- **Rust and JavaScript formatting drift** → Keep the display rule identical and assert the same expected caption text in render and JavaScript checks.
- **Untrusted EXIF text** → The model reaches templates and JSON as text; existing HTML escaping covers rendered captions, and tests should include markup characters.

## Migration Plan

1. Bump the state schema, add the model column, and clear cached probe metadata once so unchanged items are re-probed. Reuse existing thumbnails.
2. Bump the JSON schema, rebuild the catalog, and let the server keep the previous snapshot until the new complete snapshot is published.
3. Rollback requires reverting the code and rebuilding; keep a copy of the previous state database if downgrading without a rebuild is needed.

## Open Questions

- Should the text search filter also match the model? This is a separate behavior change and can be decided later.
- Which additional video model tag names are worth supporting beyond QuickTime and generic `model`? Revisit when real samples appear.
