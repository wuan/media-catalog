# Spec Delta

## MODIFIED Requirements

### Requirement: Derive the album from the current page

The viewer SHALL treat the catalog tiles rendered on the current page, in document order, as the album and SHALL restrict navigation to that album.

#### Scenario: Month page album
- **WHEN** a visitor opens a tile on `/2024/05/`
- **THEN** previous and next navigation move through that month's items only

#### Scenario: Event page album
- **WHEN** a visitor opens a tile on a sectioned event page
- **THEN** navigation moves through every item on that event page, crossing section boundaries and including the Other bucket when present

#### Scenario: Search result album
- **WHEN** a visitor opens a tile on a filtered search page
- **THEN** previous and next navigation move through only the currently visible result tiles

#### Scenario: Filtered item disappears
- **WHEN** filter changes while the viewer is open and the current item no longer matches
- **THEN** the viewer closes and the updated result grid remains visible

#### Scenario: Root recent section album
- **WHEN** a visitor opens a tile in the root page's recent section
- **THEN** navigation moves through only the recent tiles shown on that page, in their newest-first order
