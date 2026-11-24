# Spec Delta

## MODIFIED Requirements

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
