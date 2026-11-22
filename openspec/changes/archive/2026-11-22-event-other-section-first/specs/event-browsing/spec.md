# Spec Delta

## MODIFIED Requirements

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

The service SHALL render `/events/{slug}/` from the catalog snapshot as one page containing the event title, date range, description when present, item count, and the event's items grouped by authored sections when configured or by folder month otherwise. On sectioned pages, the Other bucket SHALL appear first, followed by sections in chronological order; months SHALL appear in reverse chronological order on unsectioned pages. The page SHALL include in-page navigation to its groups, with the Other link first when present, reuse the existing lazy-loaded tile presentation, and remain usable without JavaScript.

#### Scenario: Cross-month event page
- **WHEN** an event contains items from May and June and has no sections
- **THEN** the page shows June items before May items, with a month heading for each group

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

### Requirement: Event page items are oldest first

On event pages, items within the "Other" bucket, each authored section, and each folder-month group SHALL be ordered by capture timestamp ascending (oldest first), with the relative path as a stable tie-break. Because the Other bucket renders first and sections follow in chronological order, the full event page SHALL present all items in overall oldest-first order. Month, year, and index pages SHALL keep the existing newest-first item order. The in-page viewer's previous/next navigation on an event page SHALL follow the document order of the rendered tiles.

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
- **THEN** items remain newest first

#### Scenario: Timestamp tie-break
- **WHEN** two items on an event page share the same capture timestamp
- **THEN** they are ordered by relative path ascending
