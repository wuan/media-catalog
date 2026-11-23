# Spec Delta

## MODIFIED Requirements

### Requirement: Emit sharded JSON catalog data

The generator SHALL emit `catalog/index.json` describing years, counts, and shard paths directly under the configured data root, and one `catalog/YYYY.json` shard per year containing item records with paths, kind, timestamps, dimensions, duration, camera model when known, lens settings when known, geocoordinates when location publication is enabled, file size in bytes, thumbnail URL, and media URL.

#### Scenario: Year shards
- **WHEN** the catalog spans multiple years
- **THEN** each year has its own shard and the index points to each shard

#### Scenario: Record contents
- **WHEN** a shard is read
- **THEN** each item record provides enough data to render a thumbnail link, show camera settings, and open the original without additional requests

#### Scenario: Model field
- **WHEN** a source item has a known camera model
- **THEN** its shard record carries the model; when no model is known, the field is omitted

#### Scenario: File size field
- **WHEN** the generator publishes an item
- **THEN** its shard record carries `size_bytes` with the original file's size in bytes as seen by the scan

#### Scenario: Shards without file sizes
- **WHEN** the runtime loads a shard whose item records have no `size_bytes` field
- **THEN** the shard loads without error and those items are treated as having an unknown file size

#### Scenario: Lens fields
- **WHEN** an item has known aperture, exposure time, ISO, or focal length
- **THEN** its shard record carries those values; each unknown value is omitted

#### Scenario: Location records
- **WHEN** location publication is enabled and an item has coordinates
- **THEN** its shard record carries latitude and longitude

#### Scenario: Location publication disabled
- **WHEN** location publication is disabled
- **THEN** no item record carries latitude, longitude, or location fields

#### Scenario: Records without new metadata
- **WHEN** the runtime loads a shard published before lens or location fields existed
- **THEN** the shard loads without error and those items are treated as having unknown camera settings and no coordinates

#### Scenario: Data root location
- **WHEN** the generator publishes with the data root at `./data`
- **THEN** the index is written to `./data/catalog/index.json` and shards to `./data/catalog/YYYY.json`, with no `public/` segment

## ADDED Requirements

### Requirement: Expose item metadata to the viewer

Server-rendered month and event tiles, and client-rendered search tiles, SHALL expose each item's known lens settings and its published coordinates as data attributes the viewer script reads, while keeping the visible tile caption unchanged.

#### Scenario: Month and event tiles
- **WHEN** a month or event page renders a tile whose item has camera settings
- **THEN** the tile carries those values as data attributes for the viewer

#### Scenario: Search result tiles
- **WHEN** the search page renders a matching item whose shard record has camera settings
- **THEN** the generated tile carries those values as data attributes for the viewer

#### Scenario: Unknown settings
- **WHEN** an item has no aperture, exposure time, ISO, or focal length
- **THEN** the tile omits the corresponding attributes and the caption is unchanged

#### Scenario: Published coordinates only
- **WHEN** location publication is enabled and an item has coordinates
- **THEN** the tile carries the latitude and longitude as data attributes

#### Scenario: Coordinates withheld
- **WHEN** location publication is disabled or an item has no coordinates
- **THEN** the tile carries no coordinate attributes

#### Scenario: Caption unchanged
- **WHEN** a tile gains metadata attributes
- **THEN** its visible one-line caption still shows only the capture time and camera model
