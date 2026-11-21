# Thumbnail Generation Specification

## Purpose

Defines how the generator produces cacheable thumbnails for every catalog item, including images and video poster frames, so the static catalog can deliver fast previews without touching originals.

## Requirements

### Requirement: Generate a single thumbnail size
The generator SHALL produce one 512x512 WebP thumbnail per catalog item using a center cover crop that preserves aspect ratio without distortion.

#### Scenario: Landscape image
- **WHEN** a 4032x3024 photo is processed
- **THEN** its thumbnail is 512x512 WebP, cropped from the center

#### Scenario: Portrait image
- **WHEN** a 3024x4032 photo is processed
- **THEN** its thumbnail is 512x512 WebP with the subject area center-cropped, not stretched

#### Scenario: Thumbnail location
- **WHEN** an item under `2024/05/` receives a thumbnail
- **THEN** the file is written under `public/thumbs/2024/05/` with a unique name derived from the item

### Requirement: Respect source orientation
The generator SHALL apply image EXIF orientation and video rotation metadata so thumbnails appear upright.

#### Scenario: Rotated phone photo
- **WHEN** a photo carries an EXIF orientation that rotates it 90 degrees
- **THEN** the thumbnail is rendered upright matching the intended viewing orientation

#### Scenario: Rotated phone video
- **WHEN** a video carries rotation metadata
- **THEN** its poster frame is rendered upright

### Requirement: Use a representative video frame
The generator SHALL create video thumbnails from a representative frame rather than the first frame, and SHALL record the video duration for display.

#### Scenario: Normal-length video
- **WHEN** a video is longer than a few seconds
- **THEN** its thumbnail uses a frame sampled after the opening moment

#### Scenario: Very short video
- **WHEN** a video is too short to seek safely
- **THEN** the generator falls back to an available frame without aborting the build

#### Scenario: Poster generation failure
- **WHEN** a video poster frame cannot be extracted
- **THEN** the item remains cataloged, its thumbnail is reported missing, and the build continues

### Requirement: Version thumbnails by source fingerprint
The generator SHALL derive a thumbnail version from the source file's size and modification time, and SHALL expose that version in generated URLs so unchanged sources keep stable URLs and changed sources invalidate browser caches.

#### Scenario: Unchanged source
- **WHEN** a source file's size and modification time are unchanged between runs
- **THEN** its thumbnail URL remains identical

#### Scenario: Replaced source
- **WHEN** a source file at the same path changes size or modification time
- **THEN** the thumbnail is regenerated and its URL carries a new version

### Requirement: Support v1 image and video formats
The generator SHALL decode JPEG, PNG, and WebP images and SHALL use external video tooling for common MP4, MOV, and HEVC video containers; HEIC support SHALL be deferred behind a decoder boundary and reported as unsupported in v1.

#### Scenario: Common photo formats
- **WHEN** the catalog contains JPEG, PNG, or WebP images
- **THEN** all receive thumbnails without conversion of the originals

#### Scenario: Common video formats
- **WHEN** the catalog contains MP4, MOV, or HEVC videos
- **THEN** they are probed and receive poster thumbnails

#### Scenario: HEIC present in v1
- **WHEN** a HEIC photo is encountered before HEIC support is added
- **THEN** it is reported as unsupported and skipped without failing the build

### Requirement: Emit thumbnails without source metadata
The generator SHALL write thumbnails that do not carry GPS, camera, or other source metadata fields.

#### Scenario: Thumbnail of geotagged photo
- **WHEN** a geotagged source photo is thumbnailed
- **THEN** the resulting thumbnail contains no GPS metadata
