# Event Browsing Specification

## Purpose

Adds authored, time-ranged events as a second browsable structure over the media catalog while preserving the existing year and month hierarchy.

## Requirements

### Requirement: Define events in a content file

The generator SHALL read an optional authored events file whose path is declared by `events_file` in `media-catalog.toml`, or by the `--events` flag. The CLI flag SHALL override the configured path. Each event SHALL declare a stable `slug`, `title`, inclusive `start` and `end` timestamps in `YYYY-MM-DD HH:MM` form, and MAY declare a `description`.

#### Scenario: Default content file
- **WHEN** `media-catalog.toml` declares `events_file = "path/to/events.toml"` and no `--events` flag is given
- **THEN** the generator loads the events from that path

#### Scenario: Explicit content file
- **WHEN** `--events path/to/events.toml` is given while `media-catalog.toml` also declares `events_file`
- **THEN** the generator loads the events from the `--events` path

#### Scenario: No content file
- **WHEN** neither `events_file` nor `--events` is set
- **THEN** the build proceeds without events and publishes a catalog without event data

#### Scenario: Missing explicit file
- **WHEN** `--events` names a file that does not exist
- **THEN** the generator fails before scanning and reports the missing path

#### Scenario: Missing configured file
- **WHEN** `events_file` names a file that does not exist
- **THEN** the generator fails before scanning and reports the missing path

#### Scenario: Relative configured path
- **WHEN** `events_file` is a relative path
- **THEN** the generator resolves it against the current directory, like the storage root settings

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

### Requirement: Define event sections

The generator SHALL accept optional authored sections on an event. Each section SHALL declare a unique slug, a title, a bookmark timestamp `at` in `YYYY-MM-DD HH:MM` form, and MAY declare a description. Section slugs MUST use the same character rules as event slugs, sections MUST be contained by the event range, and section timestamps MUST be distinct.

#### Scenario: Sectioned event
- **WHEN** an event declares sections for Lisbon, Algarve, and Porto
- **THEN** the generator accepts them and associates each section with that event

#### Scenario: Invalid section slug
- **WHEN** a section slug contains uppercase letters, spaces, underscores, or repeated hyphens
- **THEN** the build fails before scanning and identifies the invalid section slug

#### Scenario: Duplicate section slug
- **WHEN** two sections in the same event use the same slug
- **THEN** the build fails before scanning and identifies the duplicate

#### Scenario: Section outside the event
- **WHEN** a section's `at` timestamp is before the event start or after the event end
- **THEN** the build fails before scanning

#### Scenario: Duplicate section timestamp
- **WHEN** two sections in the same event use the same `at` timestamp
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

### Requirement: Resolve bookmark sections to published ranges

The generator SHALL resolve each authored section into an inclusive `start`/`end` range in the published index. A section starts at its `at`; a section that is not last ends one minute before the next section's `at`; the last section ends at the event end. Published sections SHALL be ordered by start timestamp.

#### Scenario: Published section range
- **WHEN** Lisbon is authored at `2025-04-12 00:00` and Algarve at `2025-04-20 00:00`
- **THEN** the index publishes Lisbon as `2025-04-12 00:00` through `2025-04-19 23:59` and Algarve starting at `2025-04-20 00:00`

#### Scenario: Boundary item
- **WHEN** an item's effective timestamp is exactly `2025-04-20 00:00`
- **THEN** it belongs to Algarve rather than Lisbon

#### Scenario: Last section
- **WHEN** Porto is the last section and the event ends at `2025-05-18 23:59`
- **THEN** the published Porto range ends at `2025-05-18 23:59`

#### Scenario: Full partition
- **WHEN** the first section starts at the event start
- **THEN** the published ranges cover every minute of the event without gaps or overlap

#### Scenario: Event without sections
- **WHEN** an event declares no sections
- **THEN** its published index entry omits the section field and remains byte-identical to output before section support

#### Scenario: Repeat build
- **WHEN** the same event sections are processed twice
- **THEN** the published section data is byte-identical

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

### Requirement: Validate published event sections

The service SHALL validate event sections when loading a catalog snapshot. Section slugs MUST be unique within an event, start and end timestamps MUST parse, start MUST NOT be after end, ranges MUST be contained by the event, sections MUST be ordered, and ranges MUST NOT overlap. An invalid snapshot SHALL be rejected and the previous snapshot SHALL remain in service.

#### Scenario: Section outside the event range
- **WHEN** a published section starts before its event or ends after it
- **THEN** the reload is rejected and the previous snapshot keeps serving

#### Scenario: Overlapping sections
- **WHEN** a published section ends at or after the next section's start
- **THEN** the reload is rejected and the previous snapshot keeps serving

#### Scenario: Duplicate published section slug
- **WHEN** one event declares two sections with the same slug
- **THEN** the reload is rejected and the previous snapshot keeps serving

#### Scenario: Valid sections
- **WHEN** published section ranges are consistent
- **THEN** the new snapshot becomes available to event routes

### Requirement: Serve event browsing from the top level

The service SHALL serve an event list at `/events/` and add an Events section above the existing Years section at `/`. Events SHALL be ordered by newest start timestamp and then by slug. Each overview entry SHALL show the event title, a month-level span of its inclusive start and end, and its item count; SHALL NOT show the event description; and SHALL render the span and count as secondary text smaller than the title. Both the root section and `/events/` SHALL render the same compact entry.

#### Scenario: Root with events
- **WHEN** a published catalog contains events and a visitor requests `/`
- **THEN** the page lists compact event entries above the years hierarchy with links to their event pages

#### Scenario: Event list
- **WHEN** a visitor requests `/events/` for a catalog with events
- **THEN** the page lists each event with its title, month-level span, and item count, and does not show its description

#### Scenario: No events
- **WHEN** a visitor requests `/events/` for a catalog without events
- **THEN** the page renders an empty event list without error

#### Scenario: Catalog unavailable
- **WHEN** a visitor requests `/events/` before a catalog is published
- **THEN** the service responds with `503`

#### Scenario: Span within one month
- **WHEN** an event starts and ends in the same month of the same year, such as May 2024
- **THEN** its overview span reads `May 2024`

#### Scenario: Span across months in one year
- **WHEN** an event starts in May and ends in June of the same year
- **THEN** its overview span reads `May-Jun 2024`

#### Scenario: Span across years
- **WHEN** an event starts in December 2024 and ends in January 2025
- **THEN** its overview span reads `Dec 2024 - Jan 2025`

#### Scenario: Span across years in the same month
- **WHEN** an event starts in May 2024 and ends in May 2026
- **THEN** its overview span reads `May 2024 - May 2026`

#### Scenario: Description omitted from the overview
- **WHEN** an event declares a description and a visitor views `/` or `/events/`
- **THEN** the overview entry does not render that description, while the event's own page still does

#### Scenario: Secondary type for span and count
- **WHEN** an overview entry renders its span and item count
- **THEN** both appear in a smaller, muted type treatment than the title

### Requirement: Render unsectioned items

The service SHALL render items that fall outside every published section in an additional "Other" bucket before the authored sections, and SHALL omit that bucket when no such items exist. The Other bucket SHALL be the first group on a sectioned event page so the full page reads in chronological order.

#### Scenario: Items between sections
- **WHEN** published section ranges leave a gap containing items
- **THEN** those items appear in the Other bucket, which renders before all authored sections

#### Scenario: Items before the first section
- **WHEN** items occur before the first published section starts
- **THEN** those items appear in the Other bucket at the top of the event page

#### Scenario: No unsectioned items
- **WHEN** every item falls inside a published section
- **THEN** the Other bucket is omitted

### Requirement: Render event pages

The service SHALL render `/events/{slug}/` from the catalog snapshot as one page containing the event title, date range, description when present, item count, and the event's items grouped by authored sections when configured or by folder month otherwise. On sectioned pages, the Other bucket SHALL appear first, followed by sections in chronological order; folder-month groups SHALL appear oldest first on unsectioned pages. The page SHALL include in-page navigation to its groups, with the Other link first when present, reuse the existing lazy-loaded tile presentation, and remain usable without JavaScript.

#### Scenario: Cross-month event page
- **WHEN** an event contains items from May and June and has no sections
- **THEN** the page shows May items before June items, with a month heading for each group

#### Scenario: Sectioned event page
- **WHEN** an event has authored sections
- **THEN** the page shows the Other bucket first (when non-empty), then the sections in chronological order instead of folder-month groups

#### Scenario: Section navigation
- **WHEN** a sectioned event page is rendered
- **THEN** it includes in-page links to the Other bucket (when present, as the first link) and each non-empty section heading

#### Scenario: Month heading link
- **WHEN** an event page shows a month group
- **THEN** its heading links to the corresponding `/YYYY/MM/` catalog page

#### Scenario: Item presentation
- **WHEN** an event page renders images, videos, or items without thumbnails
- **THEN** it uses the same lazy image, video marker, placeholder, escaped metadata, and original-media link behavior as month pages

#### Scenario: Empty event
- **WHEN** an event's range matches no items
- **THEN** the event page renders its summary with an empty-state message

#### Scenario: Empty section
- **WHEN** a section's range matches no items
- **THEN** the section is omitted from both the page content and its navigation

#### Scenario: Unknown slug
- **WHEN** a visitor requests `/events/{slug}/` for a slug not in the catalog
- **THEN** the service responds with `404`

#### Scenario: Catalog unavailable
- **WHEN** a visitor requests an event page before a catalog is published
- **THEN** the service responds with `503`

#### Scenario: JavaScript disabled
- **WHEN** the browser has JavaScript disabled
- **THEN** the event page and its sections remain browsable

### Requirement: Return to the top from each event group

The service SHALL render a back-to-top link at the end of every rendered event group: the Other bucket, each authored section, and each folder-month group on unsectioned event pages. The link SHALL follow the group's tile grid and SHALL target a top anchor on the same page. It SHALL be a plain anchor that works without JavaScript.

#### Scenario: Authored section
- **WHEN** an event page renders an authored section with items
- **THEN** a back-to-top link appears after the section's tile grid

#### Scenario: Other bucket
- **WHEN** an event page renders a non-empty Other bucket
- **THEN** a back-to-top link appears after the Other bucket's tile grid

#### Scenario: Month group on an unsectioned event
- **WHEN** an unsectioned event page renders a folder-month group
- **THEN** a back-to-top link appears after that month group's tile grid

#### Scenario: Empty event
- **WHEN** an event's range matches no items
- **THEN** the page renders its empty state without back-to-top links

#### Scenario: JavaScript disabled
- **WHEN** the browser has JavaScript disabled
- **THEN** the back-to-top link still returns the visitor to the top of the page

### Requirement: Event month groups are oldest first

On unsectioned event pages, folder-month groups SHALL be ordered by folder year and month ascending (oldest first), regardless of whether the event spans one or several years. This SHALL place month groups in the same chronological direction as the items already ordered oldest first within each group.

#### Scenario: Cross-month event page
- **WHEN** an unsectioned event matches items in May and June
- **THEN** the May group renders before the June group

#### Scenario: Multi-year event page
- **WHEN** an unsectioned event spans items from two folder years
- **THEN** the earlier year's month groups render before the later year's month groups

#### Scenario: Sectioned event page unaffected
- **WHEN** an event has authored sections
- **THEN** the page continues to render sections in chronological order without folder-month groups

#### Scenario: Viewer navigation follows month groups
- **WHEN** a visitor opens the in-page viewer on an unsectioned event page and uses next/previous
- **THEN** navigation crosses month groups in the same oldest-first order the page renders

#### Scenario: Non-event pages unchanged
- **WHEN** a visitor browses a month, year, or index page
- **THEN** years remain newest first, months ascend within each year, and month items render oldest first

### Requirement: Report large events

The generator SHALL report how many items each configured event matched and SHALL warn when a single event exceeds 500 items, because a very large event is a signal to split it or introduce deeper event pages later.

#### Scenario: Large event warning
- **WHEN** an event matches more than 500 items
- **THEN** the run output includes a warning naming the event and its count

#### Scenario: Event counts
- **WHEN** a build completes with configured events
- **THEN** the run output reports each published event and its item count

### Requirement: Expose event pages to the in-page viewer

Event pages SHALL expose their tiles to the in-page viewer as one album in document order, covering every authored section and the Other bucket, while preserving direct original links for clients without the viewer script.

#### Scenario: Sectioned event album
- **WHEN** a visitor opens a tile on a sectioned event page
- **THEN** previous and next navigation move through the whole event page, crossing section boundaries in authored order

#### Scenario: Unsectioned event album
- **WHEN** a visitor opens a tile on an event page grouped by folder month
- **THEN** navigation follows the rendered month groups and item order

#### Scenario: Event tile activation
- **WHEN** a visitor activates an event tile with a plain click
- **THEN** the viewer opens in place instead of navigating away from the event page

#### Scenario: Event page without script
- **WHEN** the viewer script is unavailable
- **THEN** event tiles continue to open originals through `/media/`

### Requirement: Republish events without scanning, probing, or thumbnails

When the generator runs in events-only mode, it SHALL load the persisted item records from the state database, rebuild the catalog, reattach the authored events, and republish the catalog JSON, without reading the media root, invoking external media tools, or generating thumbnails. The republished event metadata and membership SHALL match what a full run over unchanged inputs would publish.

#### Scenario: Quick event update
- **WHEN** the events file changes and the generator runs with `--events-only`
- **THEN** `catalog/index.json` and the year shards reflect the new event metadata and membership, and no scan, probe, or thumbnail work is performed

#### Scenario: Media root unmounted
- **WHEN** the generator runs with `--events-only` while the media root does not exist
- **THEN** the run succeeds and republishes the catalog from state

#### Scenario: External tools missing
- **WHEN** the generator runs with `--events-only` and ffmpeg/ffprobe are not on `PATH`
- **THEN** the run succeeds, because no probing or thumbnail work is needed

#### Scenario: No state database
- **WHEN** `--events-only` is given before any full build has created the state database
- **THEN** the generator exits non-zero with an error stating that a full build is required first, and publishes no catalog output

#### Scenario: Dry run reports without writing
- **WHEN** `--events-only --dry-run` is given
- **THEN** the generator validates the events file and reports the planned republish without writing catalog output or state

#### Scenario: Identical output to a full run
- **WHEN** an events-only run republishes unchanged events after a full build
- **THEN** the published JSON is byte-identical to the full build's output

#### Scenario: Invalid events still fail
- **WHEN** an events-only run reads an events file that fails validation
- **THEN** the generator exits non-zero before publishing and changes no catalog output

### Requirement: Report events-only runs

The run output of an events-only run SHALL identify the run as events-only, state that scanning, probing, and thumbnail generation were skipped, and report the number of JSON files republished plus each published event and its item count.

#### Scenario: Events-only summary
- **WHEN** an events-only run completes
- **THEN** the summary marks the run as events-only and lists republished JSON files and per-event counts

### Requirement: Event page items are oldest first

On event pages, items within the "Other" bucket, each authored section, and each folder-month group SHALL be ordered by capture timestamp ascending (oldest first), with the relative path as a stable tie-break. Because the Other bucket renders first and sections follow in chronological order, the full event page SHALL present all items in overall oldest-first order. Month pages SHALL order items oldest first, year pages SHALL list months ascending, and root years SHALL remain newest first. The in-page viewer's previous/next navigation on an event page SHALL follow the document order of the rendered tiles.

#### Scenario: Sectioned event page
- **WHEN** an event has authored sections and items fall within them
- **THEN** the Other bucket (when present) renders first, followed by each section with its items oldest first

#### Scenario: Unsectioned event page
- **WHEN** an event has no sections and items are grouped by folder month
- **THEN** items within each month group are oldest first

#### Scenario: Other bucket
- **WHEN** an event page has an "Other" bucket with items outside every section
- **THEN** the Other bucket renders first on the page and its items are oldest first

#### Scenario: Viewer navigation on event page
- **WHEN** a visitor opens the in-page viewer on an event page and uses next/previous
- **THEN** navigation moves through items in the same oldest-first order as the page

#### Scenario: Non-event pages unchanged
- **WHEN** a visitor browses a month, year, or index page
- **THEN** years remain newest first, months ascend within each year, and month items render oldest first

#### Scenario: Timestamp tie-break
- **WHEN** two items on an event page share the same capture timestamp
- **THEN** they are ordered by relative path ascending
