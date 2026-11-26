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

The generator SHALL extract width, height, capture timestamp, video duration, and camera model when available, falling back to the file modification time when no capture timestamp exists. It SHALL NOT expose serial numbers, and SHALL exclude GPS and other camera details except the camera model, published lens settings, and geocoordinates when location publication is enabled. It SHALL also publish each file's size in bytes.

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
- **WHEN** a source photo contains GPS coordinates, a serial number, and camera settings, and location publication is disabled
- **THEN** GPS, location, and serial numbers do not appear in generated HTML or JSON, while the camera model, lens settings, and file size may appear

### Requirement: Order catalog items deterministically
The generator SHALL order years newest first, months within each year by month number ascending, and items within each month by capture timestamp ascending (oldest first), with the relative path as a stable tie-break, so repeated runs produce the same order.

#### Scenario: Stable ordering across runs
- **WHEN** the same unchanged media tree is scanned twice
- **THEN** the emitted pages and JSON contain items in the same order both times

#### Scenario: Same capture timestamp
- **WHEN** two items share a capture timestamp
- **THEN** their relative paths break the tie deterministically

#### Scenario: Years are newest first
- **WHEN** the catalog contains items from 2023 and 2024
- **THEN** the root index and each year's position in the published index list 2024 before 2023

#### Scenario: Months ascend within a year
- **WHEN** a year contains items in January, May, and December
- **THEN** its year page and published months array list January before May before December

#### Scenario: Items are oldest first within a month
- **WHEN** a month contains items captured on 2024-05-01, 2024-05-02, and 2024-05-03
- **THEN** the month page and shard list the 2024-05-01 item before the later items

### Requirement: Tolerate unreadable items
The generator SHALL continue processing the remaining catalog when an individual file cannot be read or decoded, and SHALL report each failure in the run summary.

#### Scenario: Corrupt image
- **WHEN** one image in a month is corrupt
- **THEN** the other items are still cataloged and the failure is listed in the summary

#### Scenario: Permission denied
- **WHEN** a file cannot be read due to permissions
- **THEN** the build continues and the run exits with a non-success status that identifies the failure count

### Requirement: Extract camera settings

The generator SHALL extract aperture, exposure time, ISO, and focal length from image EXIF metadata when those values are present, and SHALL omit each value that is absent or invalid without failing the build.

#### Scenario: Lens settings from EXIF
- **WHEN** a JPEG EXIF block records aperture f/1.8, exposure time 1/125 s, ISO 100, and focal length 26 mm
- **THEN** the item record carries all four values

#### Scenario: Partial lens settings
- **WHEN** EXIF carries only ISO and no aperture, exposure time, or focal length
- **THEN** the item carries ISO, omits the other fields, and remains valid

#### Scenario: Invalid lens values
- **WHEN** an EXIF rational for aperture, exposure time, or focal length has a zero denominator or a non-positive value
- **THEN** that value is treated as absent and the build continues

#### Scenario: Video without lens tags
- **WHEN** a video probe reports no aperture, exposure time, ISO, or focal length tags
- **THEN** the video item carries no lens fields

### Requirement: Publish geocoordinates only when enabled

The generator SHALL extract latitude and longitude when present and SHALL publish them only when location publication is enabled; otherwise coordinates SHALL be absent from every generated output.

#### Scenario: Location publication disabled
- **WHEN** a catalog is generated with location publication off
- **THEN** no latitude, longitude, or location fields appear in shard records, rendered pages, or search results

#### Scenario: Location publication enabled
- **WHEN** a catalog is generated with location publication on and an item has valid coordinates
- **THEN** that item's record carries its latitude and longitude

#### Scenario: Hemisphere references
- **WHEN** EXIF records south latitude and west longitude references
- **THEN** the published latitude and longitude are negative

#### Scenario: Invalid coordinates
- **WHEN** GPS rationals are missing, have a zero denominator, fall outside valid latitude/longitude ranges, or are both exactly zero
- **THEN** no coordinates are published for that item

#### Scenario: Video location tag
- **WHEN** a video probe reports ISO 6709 location coordinates and location publication is enabled
- **THEN** the video item carries those coordinates

### Requirement: Probe images that have no thumbnail decoder

For scanned image formats whose pixels the thumbnail pipeline cannot decode, the generator SHALL still read metadata from the container, SHALL keep the item cataloged, and SHALL mark its thumbnail unavailable without reporting a probe failure.

#### Scenario: HEIC metadata without a thumbnail
- **WHEN** a HEIC item is scanned and the thumbnail pipeline cannot decode it
- **THEN** the item remains cataloged with any EXIF capture time, model, lens settings, and published coordinates, and its thumbnail is unavailable without a probe failure

#### Scenario: HEIC without EXIF
- **WHEN** a HEIC item has no readable EXIF timestamp
- **THEN** the modification time is used and no metadata fields are published

#### Scenario: Unsupported thumbnail reported
- **WHEN** a HEIC item cannot be thumbnailed
- **THEN** it is counted as unsupported in the run summary and the build continues

### Requirement: Report probe failures by cause

The generator SHALL report probe failures separately from other errors in the run summary, with the number of failed and attempted probes. It SHALL group failures by their root cause and show, for each cause, its count and a bounded number of sample paths, ordered by count from highest to lowest.

#### Scenario: Probe failure line
- **WHEN** a run attempts to probe 10 items and 3 probes fail
- **THEN** the summary contains `Probe failures: 3 of 10`, separate from the thumbnail counts and the overall error count

#### Scenario: Failures grouped by cause
- **WHEN** 500 probes fail because the files cannot be opened due to permissions and 2 probes fail because the files are corrupt
- **THEN** the summary lists the permission cause first with its count of 500 and a few sample paths, followed by the corrupt-file cause with its count of 2, instead of only listing individual files

#### Scenario: No probe failures
- **WHEN** every attempted probe succeeds
- **THEN** the summary reports `Probe failures: 0 of <attempted>` and lists no failure causes

#### Scenario: Nothing to probe
- **WHEN** a run has no items that need probing
- **THEN** the summary reports `Probe failures: 0 of 0` and prints no systemic probe warning

### Requirement: Warn on systemic probe failure

The generator SHALL print a prominent warning when at least 4 probes were attempted and at least half of them failed. The warning SHALL say that the originals are probably unreadable in this environment and that the published catalog will lack dimensions and camera metadata for those items.

#### Scenario: Every probe fails
- **WHEN** a run attempts to probe 72,946 items and all of them fail with the same cause
- **THEN** the summary prints the systemic probe warning naming the failed and attempted counts, and the run exits non-zero

#### Scenario: Isolated failures
- **WHEN** a run attempts to probe 100 items and 2 fail
- **THEN** the failures are reported by cause but no systemic probe warning is printed

#### Scenario: Too few probes to judge
- **WHEN** a run attempts to probe 3 items and all of them fail
- **THEN** the failures are reported by cause but no systemic probe warning is printed
