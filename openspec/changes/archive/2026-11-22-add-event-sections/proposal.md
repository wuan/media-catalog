# Proposal

## Why

Long events such as a five-week Portugal trip contain enough items that a
month-grouped event page is hard to scan: the folder month is too coarse to
show the trip's chapters. Authors need a lightweight way to add bookmarks such
as "Lisbon", "Algarve", and "Porto" so visitors can understand and navigate the
trip without turning each chapter into a separate event.

## What Changes

- Add optional authored `sections` to each event in `events.toml`. Each section
  has a stable slug, title, bookmark timestamp (`at`), and optional description.
- Treat authoring as bookmarks: a section runs from its `at` timestamp until the
  next section's `at`, and the last section runs to the event end.
- Resolve every section to an inclusive `start`/`end` range in
  `catalog/index.json`, so the published contract carries concrete ranges rather
  than bookmark semantics.
- Render sectioned event pages with an in-page navigation list and one section
  per authored bookmark. Events without sections keep the existing
  month-grouped page and byte-identical output.
- Collect items that fall outside every published section into an optional
  "Other" bucket, which stays empty for bookmark-mode sections.
- Validate published section ranges during server snapshot loading: sections
  must be ordered, contained by the event, and must not overlap.
- Leave room for a later optional `end` on authored sections. Explicit ends are
  out of scope for this change, but the published range contract and generic
  renderer make them a generator-only extension later.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `event-browsing`: Adds authored event sections, resolved section ranges in the
  published index, sectioned event rendering, unsectioned-item handling, and
  section validation while preserving the existing month-grouped behavior for
  events without sections.

## Impact

- Authoring: `events.toml` gains optional `[[events.sections]]` entries.
- Generator: event parsing and validation in `src/events.rs`; section range
  resolution and publication.
- Catalog model: event metadata and views gain resolved sections.
- JSON contract: optional `sections` field on index events; shards and item
  membership remain unchanged.
- Server: snapshot validation and event rendering in `src/server/catalog.rs`
  and `src/render.rs`; `templates/event.html` gains section navigation and
  sections.
- Documentation and tests: events configuration, bookmark boundaries, resolved
  ranges, section rendering, unsectioned items, and deterministic output.
- Compatibility: catalogs without sections remain byte-identical; older servers
  ignore the additive `sections` field; no media or shard migration is required.
