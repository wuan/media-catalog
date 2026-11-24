# Spec Delta

## MODIFIED Requirements

### Requirement: Caption the open item

The viewer SHALL show the capture caption as the left segment of a single status row for the open item, containing the capture timestamp, then the camera model when known, then the original's pixel dimensions in display orientation and megapixel count when known, then the original's file size when known, SHALL NOT show the filename in the caption, and SHALL use that capture segment as the dialog's accessible name.

#### Scenario: Caption content
- **WHEN** an image item opens in the viewer with a known camera model, known dimensions, and a known file size
- **THEN** the left segment of the status row reads `2024-05-01 10:00:00 · iPhone 15 Pro · 4000x3000 12.0MP · 3.4 MB`, and it does not include the filename

#### Scenario: Caption without a model
- **WHEN** an item without a known camera model opens in the viewer
- **THEN** the left segment shows the capture timestamp followed by the dimension group when known and the file size when known

#### Scenario: Caption without a file size
- **WHEN** an item opens whose catalog record carries no file size, for example from a catalog published before sizes were recorded
- **THEN** the left segment omits the size and shows the remaining parts unchanged, ending with the dimension group when known

#### Scenario: Caption without dimensions
- **WHEN** an item opens whose catalog record carries no width or no height
- **THEN** the left segment omits the whole `<width>x<height> <megapixels>MP` group and its separator, showing the remaining parts unchanged

#### Scenario: Dimension group content
- **WHEN** the caption shows the dimension group
- **THEN** it reads the two pixel dimensions joined by a bare `x` followed by a space, the megapixel count, and the label `MP`, such as `4000x3000 12.0MP`

#### Scenario: Video dimensions
- **WHEN** a video item opens with known frame dimensions
- **THEN** the caption shows the same dimension group as an image item, such as `1920x1080 2.1MP`, before the file size

#### Scenario: Megapixel precision
- **WHEN** the caption shows a megapixel count
- **THEN** it always shows exactly one decimal place, so 4,000x3,000 reads `12.0MP`, 4,032x3,024 reads `12.2MP`, and 640x480 reads `0.3MP`

#### Scenario: File size units
- **WHEN** the caption shows a file size
- **THEN** it uses decimal units: whole bytes as `B` below 1 kB, whole kilobytes as `kB` below 1 MB, and one decimal place as `MB` below 1 GB or as `GB` from 1 GB up, so 845 bytes reads `845 B`, 532,000 bytes reads `532 kB`, 3,400,000 bytes reads `3.4 MB`, and 1,240,000,000 bytes reads `1.2 GB`

#### Scenario: Rounding never shows a full unit in the smaller unit
- **WHEN** rounding would display 1000 of a unit, for example 999,950 bytes
- **THEN** the caption shows the next larger unit instead, such as `1.0 MB`
