# Spec Delta

## MODIFIED Requirements

### Requirement: Generate a single thumbnail size

The generator SHALL produce one 512x512 WebP thumbnail per catalog item under the configured data root using a center cover crop that preserves aspect ratio without distortion.

#### Scenario: Landscape image
- **WHEN** a 4032x3024 photo is processed
- **THEN** its thumbnail is 512x512 WebP, cropped from the center

#### Scenario: Portrait image
- **WHEN** a 3024x4032 photo is processed
- **THEN** its thumbnail is 512x512 WebP with the subject area center-cropped, not stretched

#### Scenario: Thumbnail location
- **WHEN** an item under `2024/05/` receives a thumbnail and the data root is `./data`
- **THEN** the file is written under `./data/thumbs/2024/05/` with a unique name derived from the item
