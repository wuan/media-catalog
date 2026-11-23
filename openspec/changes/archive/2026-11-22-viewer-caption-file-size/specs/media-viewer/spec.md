# Spec Delta

## MODIFIED Requirements

### Requirement: Caption the open item

The viewer SHALL show a single caption line for the open item containing the capture timestamp, then the camera model when known, then the original's file size when known, SHALL NOT show the filename in the caption, and SHALL use that caption line as the dialog's accessible name.

#### Scenario: Caption content
- **WHEN** an item opens in the viewer
- **THEN** the caption shows the capture timestamp followed by the camera model when known and the file size when known, and does not show the filename

#### Scenario: Caption without a model
- **WHEN** an item without a known camera model opens in the viewer
- **THEN** the caption shows the capture timestamp followed by the file size when known

#### Scenario: Caption without a file size
- **WHEN** an item opens whose catalog record carries no file size, for example from a catalog published before sizes were recorded
- **THEN** the caption omits the size and shows the remaining parts unchanged

#### Scenario: File size units
- **WHEN** the caption shows a file size
- **THEN** it uses decimal units: whole bytes as `B` below 1 kB, whole kilobytes as `kB` below 1 MB, and one decimal place as `MB` below 1 GB or as `GB` from 1 GB up, so 845 bytes reads `845 B`, 532,000 bytes reads `532 kB`, 3,400,000 bytes reads `3.4 MB`, and 1,240,000,000 bytes reads `1.2 GB`

#### Scenario: Rounding never shows a full unit in the smaller unit
- **WHEN** rounding would display 1000 of a unit, for example 999,950 bytes
- **THEN** the caption shows the next larger unit instead, such as `1.0 MB`
