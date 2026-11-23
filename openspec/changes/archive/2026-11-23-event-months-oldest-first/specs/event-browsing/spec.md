# Spec Delta

## MODIFIED Requirements

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

## ADDED Requirements

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
- **THEN** months and items remain newest first
