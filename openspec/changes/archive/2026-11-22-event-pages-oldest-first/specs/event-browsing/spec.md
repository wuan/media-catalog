# Spec Delta

## ADDED Requirements

### Requirement: Event page items are oldest first

On event pages, items within each authored section, each folder-month group, and the
"Other" bucket SHALL be ordered by capture timestamp ascending (oldest first), with
the relative path as a stable tie-break. Month, year, and index pages SHALL keep the
existing newest-first item order. The in-page viewer's previous/next navigation on an
event page SHALL follow the document order of the rendered tiles.

#### Scenario: Sectioned event page
- **WHEN** an event has authored sections and items fall within them
- **THEN** each section renders its items oldest first

#### Scenario: Unsectioned event page
- **WHEN** an event has no sections and items are grouped by folder month
- **THEN** items within each month group are oldest first

#### Scenario: Other bucket
- **WHEN** an event page has an "Other" bucket with items outside every section
- **THEN** the Other bucket renders its items oldest first

#### Scenario: Viewer navigation on event page
- **WHEN** a visitor opens the in-page viewer on an event page and uses next/previous
- **THEN** navigation moves through items in the same oldest-first order as the page

#### Scenario: Non-event pages unchanged
- **WHEN** a visitor browses a month, year, or index page
- **THEN** items remain newest first

#### Scenario: Timestamp tie-break
- **WHEN** two items on an event page share the same capture timestamp
- **THEN** they are ordered by relative path ascending
