# Spec Delta

## ADDED Requirements

### Requirement: Advertise the location preview setting in static output

The generator SHALL advertise the resolved location preview setting in published output so the viewer can learn it without extra requests. When enabled, generated month and event pages SHALL carry `data-location-preview="true"` on the body element and `catalog/index.json` SHALL carry a `location_preview: true` field. When disabled, neither SHALL carry the setting. The search page SHALL apply the index value to the same body attribute before the viewer can preview a coordinate.

#### Scenario: Month and event pages carry the setting
- **WHEN** the location preview is enabled and a month or event page is generated
- **THEN** its body element carries the preview setting for the viewer script

#### Scenario: Index carries the setting
- **WHEN** the location preview is enabled
- **THEN** `catalog/index.json` carries `location_preview: true`

#### Scenario: Default output is unchanged
- **WHEN** the location preview is disabled
- **THEN** no page body attribute and no index field advertises it

#### Scenario: Search page propagates the setting
- **WHEN** the search page loads an index with the location preview enabled
- **THEN** it applies the setting to the page before any coordinate can be previewed

#### Scenario: Older output reads as disabled
- **WHEN** the runtime loads pages or an index published before the setting existed
- **THEN** the viewer makes no map request until the visitor activates the location link

#### Scenario: Setting without coordinates
- **WHEN** the location preview is enabled and an item has no published coordinates
- **THEN** no preview is offered for that item and no map data is requested

#### Scenario: Deterministic output
- **WHEN** the generator runs twice against an unchanged catalog with the same settings
- **THEN** the advertised setting is identical in both runs
