# Spec Delta

## ADDED Requirements

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

### Requirement: Render unsectioned items

The service SHALL render items that fall outside every published section in an additional "Other" bucket after the authored sections, and SHALL omit that bucket when no such items exist.

#### Scenario: Items between sections
- **WHEN** published section ranges leave a gap containing items
- **THEN** those items appear in the Other bucket

#### Scenario: Items before the first section
- **WHEN** items occur before the first published section starts
- **THEN** those items appear in the Other bucket

#### Scenario: No unsectioned items
- **WHEN** every item falls inside a published section
- **THEN** the Other bucket is omitted

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

## MODIFIED Requirements

### Requirement: Render event pages

The service SHALL render `/events/{slug}/` from the catalog snapshot as one page containing the event title, date range, description when present, item count, and the event's items grouped by authored sections when configured or by folder month otherwise. Sections SHALL appear in chronological order; months SHALL appear in reverse chronological order. The page SHALL include in-page navigation to its sections, reuse the existing lazy-loaded tile presentation, and remain usable without JavaScript.

#### Scenario: Cross-month event page
- **WHEN** an event contains items from May and June and has no sections
- **THEN** the page shows June items before May items, with a month heading for each group

#### Scenario: Sectioned event page
- **WHEN** an event has authored sections
- **THEN** the page shows the sections in chronological order instead of folder-month groups

#### Scenario: Section navigation
- **WHEN** a sectioned event page is rendered
- **THEN** it includes in-page links to each non-empty section heading

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
