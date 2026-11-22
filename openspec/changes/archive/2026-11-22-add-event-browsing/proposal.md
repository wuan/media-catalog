# Proposal

## Why

The catalog currently exposes only the archive's `YYYY/MM` folder hierarchy, so
there is no way to browse a real-world event that crosses month boundaries, such
as a trip or a season. Adding authored events gives visitors a second, curated
way to explore the same archive without changing the existing year/month views.

## What Changes

- Add an `events.toml` content file read only by the generator. Each event
  declares a stable `slug`, `title`, inclusive `start` and `end` timestamps at
  minute precision, and an optional `description`.
- Assign catalog items to events by their effective timestamp (`captured_at`,
  falling back to mtime). Membership is independent of the folder hierarchy, so
  an item can belong to multiple overlapping events.
- Publish event metadata in `catalog/index.json` and an `events` slug list on
  each item in the year shards. The additions are optional and do not bump the
  JSON schema version.
- Validate event definitions and published membership counts so an invalid
  config or inconsistent catalog fails before publication or snapshot reload.
- Serve `/events/` and `/events/{slug}/` from the catalog snapshot, and add an
  Events section above Years on the root page.
- Render an event page as a summary plus month-grouped item sections, reusing
  the existing lazy-loading, escaped tile markup and no-JavaScript behavior.
- Keep search unchanged for now; the published item data already carries event
  slugs, so an event filter can be added later without a schema change.

## Capabilities

### New Capabilities

- `event-browsing`: Defines authored events, timestamp-based membership,
  published event data, and the event browsing routes and pages.

### Modified Capabilities

_None._ Existing year/month browsing, inventory, JSON and server requirements
remain valid because event data and routes are additive.

## Impact

- Generator configuration: new `events.toml` loader and `--events` flag.
- Catalog model: event attachment and event views derived from item timestamps.
- JSON contract: optional `events` fields in the index and year shards.
- Server: snapshot validation, `/events/` and `/events/{slug}/` routes, root
  page section, and shared rendering of event month sections.
- Documentation and tests: event configuration, time semantics, routes, JSON
  examples, validation failures, and large-event guidance.
- Compatibility: catalogs without configured events produce byte-identical
  output to today; older servers ignore the new optional fields.
