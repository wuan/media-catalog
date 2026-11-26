# Spec Delta

## MODIFIED Requirements

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
