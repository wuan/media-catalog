# Spec Delta

## MODIFIED Requirements

### Requirement: Keep month pages lightweight

The runtime SHALL lazy-load month thumbnails and SHALL NOT emit a separate HTML page per item.

#### Scenario: Large month
- **WHEN** a month contains roughly 500 items
- **THEN** the month page references all items but defers off-screen thumbnail loading

#### Scenario: Item interaction
- **WHEN** a visitor selects an item on a month page and the viewer script is available
- **THEN** the item opens in the in-page viewer while the original remains reachable through its link

#### Scenario: Item interaction without script
- **WHEN** a visitor selects an item and the viewer script is unavailable
- **THEN** the original opens directly from `/media/` rather than a generated detail page
