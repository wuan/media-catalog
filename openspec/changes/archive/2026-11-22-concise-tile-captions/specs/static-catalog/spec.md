# Spec Delta

## MODIFIED Requirements

### Requirement: Emit sharded JSON catalog data
The generator SHALL emit `catalog/index.json` describing years, counts, and shard paths directly under the configured data root, and one `catalog/YYYY.json` shard per year containing item records with paths, kind, timestamps, dimensions, duration, camera model when known, thumbnail URL, and media URL.

#### Scenario: Year shards
- **WHEN** the catalog spans multiple years
- **THEN** each year has its own shard and the index points to each shard

#### Scenario: Record contents
- **WHEN** a shard is read
- **THEN** each item record provides enough data to render a thumbnail link and open the original without additional requests

#### Scenario: Model field
- **WHEN** a source item has a known camera model
- **THEN** its shard record carries the model; when no model is known, the field is omitted

#### Scenario: Data root location
- **WHEN** the generator publishes with the data root at `./data`
- **THEN** the index is written to `./data/catalog/index.json` and shards to `./data/catalog/YYYY.json`, with no `public/` segment

## ADDED Requirements

### Requirement: Present concise tile captions
The runtime SHALL render every month, event, and search grid tile with a single-line caption containing the capture time as `YY-MM-DD HH:MM` and, when known, the camera model. The caption SHALL omit the visible filename, stay on one line, and keep the item name in the thumbnail's alternative text.

#### Scenario: Caption with a model
- **WHEN** an item captured at `2024-05-01 10:00:00` has the model `iPhone 15 Pro`
- **THEN** its tile caption reads `24-05-01 10:00 iPhone 15 Pro`

#### Scenario: Caption without a model
- **WHEN** an item has no known model
- **THEN** its tile caption reads only `YY-MM-DD HH:MM`, such as `24-05-01 10:00`

#### Scenario: All grid surfaces
- **WHEN** a month page, an event page, or a search result grid is rendered
- **THEN** each tile uses the same one-line caption format

#### Scenario: Long model text
- **WHEN** the model text is wider than the tile
- **THEN** the caption remains one line and truncates instead of wrapping

#### Scenario: Accessibility preserved
- **WHEN** a tile is rendered without a visible filename caption
- **THEN** its thumbnail alternative text still derives from the item name or capture date
