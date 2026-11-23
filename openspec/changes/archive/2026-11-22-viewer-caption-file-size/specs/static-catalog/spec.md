# Spec Delta

## MODIFIED Requirements

### Requirement: Emit sharded JSON catalog data

The generator SHALL emit `catalog/index.json` describing years, counts, and shard paths directly under the configured data root, and one `catalog/YYYY.json` shard per year containing item records with paths, kind, timestamps, dimensions, duration, camera model when known, file size in bytes, thumbnail URL, and media URL.

#### Scenario: Year shards
- **WHEN** the catalog spans multiple years
- **THEN** each year has its own shard and the index points to each shard

#### Scenario: Record contents
- **WHEN** a shard is read
- **THEN** each item record provides enough data to render a thumbnail link and open the original without additional requests

#### Scenario: Model field
- **WHEN** a source item has a known camera model
- **THEN** its shard record carries the model; when no model is known, the field is omitted

#### Scenario: File size field
- **WHEN** the generator publishes an item
- **THEN** its shard record carries `size_bytes` with the original file's size in bytes as seen by the scan

#### Scenario: Shards without file sizes
- **WHEN** the runtime loads a shard whose item records have no `size_bytes` field
- **THEN** the shard loads without error and those items are treated as having an unknown file size

#### Scenario: Data root location
- **WHEN** the generator publishes with the data root at `./data`
- **THEN** the index is written to `./data/catalog/index.json` and shards to `./data/catalog/YYYY.json`, with no `public/` segment
