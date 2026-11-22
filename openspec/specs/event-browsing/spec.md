# Event Browsing Specification

## Purpose

Adds authored, time-ranged events as a second browsable structure over the media catalog while preserving the existing year and month hierarchy.

## Requirements

### Requirement: Define events in a content file

The generator SHALL read an optional `events.toml` content file containing authored events. Each event SHALL declare a stable `slug`, `title`, inclusive `start` and `end` timestamps in `YYYY-MM-DD HH:MM` form, and MAY declare a `description`.

#### Scenario: Default content file
- **WHEN** `./events.toml` exists and no `--events` path is given
- **THEN** the generator loads the events from `./events.toml`

#### Scenario: Explicit content file
- **WHEN** `--events path/to/events.toml` is given
- **THEN** the generator loads the events from that path

#### Scenario: No content file
- **WHEN** no `events.toml` exists and no `--events` path is given
- **THEN** the build proceeds without events and publishes a catalog without event data

#### Scenario: Missing explicit file
- **WHEN** `--events` names a file that does not exist
- **THEN** the generator fails before scanning and reports the missing path

### Requirement: Validate event definitions

The generator SHALL reject event definitions that cannot produce stable event pages. Slugs MUST be unique and use lowercase ASCII letters, digits, and single hyphens; timestamps MUST parse; `start` MUST NOT be after `end`; and each event MUST have a title.

#### Scenario: Duplicate slug
- **WHEN** two events declare the same slug
- **THEN** the build fails before scanning and identifies the duplicate

#### Scenario: Malformed slug
- **WHEN** an event slug contains uppercase letters, spaces, underscores, or other unsupported characters
- **THEN** the build fails before scanning

#### Scenario: Reversed range
- **WHEN** an event's `start` is later than its `end`
- **THEN** the build fails before scanning

#### Scenario: Unparseable timestamp
- **WHEN** an event timestamp is not in `YYYY-MM-DD HH:MM` form
- **THEN** the build fails before scanning

### Requirement: Assign items by effective timestamp

The generator SHALL assign each catalog item to every event whose inclusive range contains the item's effective timestamp, truncated to minute precision. Event membership SHALL NOT replace the folder-derived year and month grouping used by the existing hierarchy.

#### Scenario: Start boundary
- **WHEN** an event starts at `2024-05-28 09:00` and an item's effective timestamp is `2024-05-28 09:00:00`
- **THEN** the item belongs to the event

#### Scenario: End minute
- **WHEN** an event ends at `2024-06-05 18:30` and an item's effective timestamp is `2024-06-05 18:30:45`
- **THEN** the item belongs to the event

#### Scenario: Outside the range
- **WHEN** an item's effective timestamp is before the start or after the end minute
- **THEN** the item does not belong to the event

#### Scenario: Cross-month event
- **WHEN** an event spans May and June and items occur in both months
- **THEN** each matching item belongs to the event regardless of its folder month

#### Scenario: Folder month differs from timestamp
- **WHEN** an item is stored under `2024/06` but its effective timestamp is `2024-05-20`
- **THEN** its event membership follows the timestamp while its year and month browsing remains `2024/06`

#### Scenario: Overlapping events
- **WHEN** an item's timestamp falls inside two configured events
- **THEN** the item belongs to both events

#### Scenario: Fallback timestamp
- **WHEN** an item has no capture timestamp and its modification time falls inside an event range
- **THEN** the item belongs to the event

### Requirement: Publish event metadata and membership

The generator SHALL publish event metadata in `catalog/index.json` and the event slugs for each item in its year shard. Event fields SHALL be omitted when no events are configured, and repeated builds over unchanged inputs SHALL produce identical event data.

#### Scenario: Event metadata
- **WHEN** an event is configured and the catalog is published
- **THEN** `catalog/index.json` includes its slug, title, start, end, optional description, and item count

#### Scenario: Item membership
- **WHEN** an item belongs to one or more events
- **THEN** its shard record lists those event slugs in a deterministic order

#### Scenario: No events configured
- **WHEN** no events are configured
- **THEN** the published JSON omits event fields and matches the catalog produced before event support

#### Scenario: Repeat build
- **WHEN** the same events, media, and metadata are processed twice
- **THEN** the event-related JSON is byte-identical

### Requirement: Validate published event data

The server SHALL validate event metadata and item membership when loading a catalog snapshot. A snapshot whose index declares duplicate slugs, whose items reference undeclared slugs, or whose declared counts disagree with the shards SHALL be rejected, and the previous complete snapshot SHALL remain in service.

#### Scenario: Undeclared slug
- **WHEN** a shard item references an event slug absent from the index
- **THEN** the reload is rejected and the previous snapshot keeps serving

#### Scenario: Count mismatch
- **WHEN** an event's declared count differs from its item membership in the shards
- **THEN** the reload is rejected and the previous snapshot keeps serving

#### Scenario: Valid event data
- **WHEN** event metadata and memberships are consistent
- **THEN** the new snapshot becomes available to all catalog routes

### Requirement: Serve event browsing from the top level

The service SHALL serve an event list at `/events/` and add an Events section above the existing Years section at `/`. Events SHALL be ordered by newest start timestamp and then by slug.

#### Scenario: Root with events
- **WHEN** a published catalog contains events and a visitor requests `/`
- **THEN** the page lists the events above the years hierarchy with links to their event pages

#### Scenario: Event list
- **WHEN** a visitor requests `/events/` for a catalog with events
- **THEN** the page lists each event with its title, date range, and item count

#### Scenario: No events
- **WHEN** a visitor requests `/events/` for a catalog without events
- **THEN** the page renders an empty event list without error

#### Scenario: Catalog unavailable
- **WHEN** a visitor requests `/events/` before a catalog is published
- **THEN** the service responds with `503`

### Requirement: Render event pages

The service SHALL render `/events/{slug}/` from the catalog snapshot as one page containing the event title, date range, description when present, item count, and the event's items grouped by folder month in reverse chronological order. Event pages SHALL reuse the existing lazy-loaded tile presentation and SHALL remain usable without JavaScript.

#### Scenario: Cross-month event page
- **WHEN** an event contains items from May and June
- **THEN** the page shows June items before May items, with a month heading for each group

#### Scenario: Month heading link
- **WHEN** an event page shows a month group
- **THEN** its heading links to the corresponding `/YYYY/MM/` catalog page

#### Scenario: Item presentation
- **WHEN** an event page renders images, videos, or items without thumbnails
- **THEN** it uses the same lazy image, video marker, placeholder, escaped metadata, and original-media link behavior as month pages

#### Scenario: Empty event
- **WHEN** an event's range matches no items
- **THEN** the event page renders its summary with an empty-state message

#### Scenario: Unknown slug
- **WHEN** a visitor requests `/events/{slug}/` for a slug not in the catalog
- **THEN** the service responds with `404`

#### Scenario: Catalog unavailable
- **WHEN** a visitor requests an event page before a catalog is published
- **THEN** the service responds with `503`

#### Scenario: JavaScript disabled
- **WHEN** the browser has JavaScript disabled
- **THEN** the event page and its month sections remain browsable

### Requirement: Report large events

The generator SHALL report how many items each configured event matched and SHALL warn when a single event exceeds 500 items, because a very large event is a signal to split it or introduce deeper event pages later.

#### Scenario: Large event warning
- **WHEN** an event matches more than 500 items
- **THEN** the run output includes a warning naming the event and its count

#### Scenario: Event counts
- **WHEN** a build completes with configured events
- **THEN** the run output reports each published event and its item count
