# Spec Delta

## MODIFIED Requirements

### Requirement: Emit sharded JSON catalog data

The generator SHALL emit `catalog/index.json` describing years, counts, and shard paths directly under the configured data root, and one `catalog/YYYY.json` shard per year containing item records with paths, kind, timestamps, dimensions, duration, thumbnail URL, and media URL.

#### Scenario: Year shards
- **WHEN** the catalog spans multiple years
- **THEN** each year has its own shard and the index points to each shard

#### Scenario: Record contents
- **WHEN** a shard is read
- **THEN** each item record provides enough data to render a thumbnail link and open the original without additional requests

#### Scenario: Data root location
- **WHEN** the generator publishes with the data root at `./data`
- **THEN** the index is written to `./data/catalog/index.json` and shards to `./data/catalog/YYYY.json`, with no `public/` segment
