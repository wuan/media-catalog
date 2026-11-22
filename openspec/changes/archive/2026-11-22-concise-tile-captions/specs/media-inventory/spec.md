# Spec Delta

## MODIFIED Requirements

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
