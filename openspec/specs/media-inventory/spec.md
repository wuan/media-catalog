# Media Inventory Specification

## Purpose

Defines how the generator discovers media, groups it by the existing `YYYY/MM` folder structure, classifies items, extracts safe metadata, and produces a deterministic catalog model for rendering and search.

## Requirements

### Requirement: Scan the media tree by folder structure
The generator SHALL recursively discover regular media files under `<media-root>/YYYY/MM/**` and SHALL treat the folder path as the authoritative year and month grouping, regardless of embedded capture dates.

#### Scenario: Discover supported media
- **WHEN** the media root contains photos and videos under `2024/05/`
- **THEN** each supported file is included in the catalog with year `2024` and month `05`

#### Scenario: Folder date differs from capture date
- **WHEN** a file stored in `2023/12/` has an EXIF capture date in January 2024
- **THEN** it remains grouped under 2023/12, while its capture date is used only for ordering and display

#### Scenario: Ignore non-media and hidden files
- **WHEN** the tree contains hidden files, sidecar files, or unsupported extensions
- **THEN** those files are skipped without affecting the catalog

### Requirement: Classify media as image or video
The generator SHALL classify each discovered file as an image or a video based on its extension and SHALL expose that classification to the catalog and search output.

#### Scenario: Mixed month
- **WHEN** a month contains both JPEG photos and MP4 videos
- **THEN** each item is classified correctly and the month's item list contains both kinds

#### Scenario: Unsupported format reported
- **WHEN** a file uses a format that v1 cannot decode, such as HEIC
- **THEN** it is skipped, reported in the run summary, and does not abort the build

### Requirement: Extract safe capture metadata
The generator SHALL extract width, height, capture timestamp when available, video duration, and camera model when available; it SHALL fall back to the file modification time when no capture timestamp exists and SHALL NOT expose GPS, location, serial numbers, or other camera details in generated output.

#### Scenario: Photo with EXIF timestamp
- **WHEN** a JPEG contains an EXIF capture timestamp
- **THEN** that timestamp is recorded for ordering and display

#### Scenario: Photo without EXIF timestamp
- **WHEN** an image has no usable capture timestamp
- **THEN** the file modification time is used instead

#### Scenario: Video metadata
- **WHEN** a video is probed successfully
- **THEN** its duration, dimensions, and any available model tag are recorded for display and thumbnail generation

#### Scenario: Photo with EXIF model
- **WHEN** a JPEG contains a non-empty EXIF `Model` value
- **THEN** that model is recorded for tile captions

#### Scenario: Video with model tag
- **WHEN** a video's probe output contains a non-empty model tag
- **THEN** that model is recorded for tile captions

#### Scenario: Privacy of metadata
- **WHEN** a source photo contains GPS coordinates, a serial number, and camera details
- **THEN** GPS, location, serial numbers, and other camera details do not appear in generated HTML or JSON, while the camera model may appear in tile captions and item records

### Requirement: Order catalog items deterministically
The generator SHALL present years, months, and items in reverse chronological order, ordering items within a month by capture timestamp and then by relative path so repeated runs produce the same order.

#### Scenario: Stable ordering across runs
- **WHEN** the same unchanged media tree is scanned twice
- **THEN** the emitted pages and JSON contain items in the same order both times

#### Scenario: Same capture timestamp
- **WHEN** two items share a capture timestamp
- **THEN** their relative paths break the tie deterministically

### Requirement: Tolerate unreadable items
The generator SHALL continue processing the remaining catalog when an individual file cannot be read or decoded, and SHALL report each failure in the run summary.

#### Scenario: Corrupt image
- **WHEN** one image in a month is corrupt
- **THEN** the other items are still cataloged and the failure is listed in the summary

#### Scenario: Permission denied
- **WHEN** a file cannot be read due to permissions
- **THEN** the build continues and the run exits with a non-success status that identifies the failure count
