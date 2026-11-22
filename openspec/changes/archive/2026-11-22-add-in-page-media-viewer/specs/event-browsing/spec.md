# Spec Delta

## ADDED Requirements

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
