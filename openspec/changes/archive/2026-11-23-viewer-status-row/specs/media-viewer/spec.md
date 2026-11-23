# Spec Delta

## MODIFIED Requirements

### Requirement: Caption the open item

The viewer SHALL show the capture caption as the left segment of a single status row for the open item, containing the capture timestamp, then the camera model when known, then the original's file size when known, SHALL NOT show the filename in the caption, and SHALL use that capture segment as the dialog's accessible name.

#### Scenario: Caption content
- **WHEN** an item opens in the viewer
- **THEN** the left segment of the status row shows the capture timestamp followed by the camera model when known and the file size when known, and does not show the filename

#### Scenario: Caption without a model
- **WHEN** an item without a known camera model opens in the viewer
- **THEN** the left segment shows the capture timestamp followed by the file size when known

#### Scenario: Caption without a file size
- **WHEN** an item opens whose catalog record carries no file size, for example from a catalog published before sizes were recorded
- **THEN** the left segment omits the size and shows the remaining parts unchanged

#### Scenario: File size units
- **WHEN** the caption shows a file size
- **THEN** it uses decimal units: whole bytes as `B` below 1 kB, whole kilobytes as `kB` below 1 MB, and one decimal place as `MB` below 1 GB or as `GB` from 1 GB up, so 845 bytes reads `845 B`, 532,000 bytes reads `532 kB`, 3,400,000 bytes reads `3.4 MB`, and 1,240,000,000 bytes reads `1.2 GB`

#### Scenario: Rounding never shows a full unit in the smaller unit
- **WHEN** rounding would display 1000 of a unit, for example 999,950 bytes
- **THEN** the caption shows the next larger unit instead, such as `1.0 MB`

### Requirement: Show camera settings in the viewer

The viewer SHALL show the camera settings as the right segment of the single status row for the open item, with aperture, exposure time, ISO, and focal length when known, using fixed units and ` · ` separators, and SHALL omit each unknown value without leaving empty separators.

#### Scenario: Complete settings
- **WHEN** an item has aperture f/1.8, exposure time 1/125 s, ISO 100, and focal length 26 mm
- **THEN** the right segment reads `26 mm · f/1.8 · 1/125 s · ISO 100`

#### Scenario: Fractional exposure below one second
- **WHEN** the exposure time is below one second
- **THEN** it is shown as a reduced fraction of a second, such as `1/125 s`

#### Scenario: Exposure of one second or longer
- **WHEN** the exposure time is at least one second
- **THEN** it is shown in seconds with up to one decimal place, such as `4 s` or `2.5 s`

#### Scenario: Aperture and focal length formatting
- **WHEN** aperture or focal length is a whole number
- **THEN** it is shown without a trailing decimal, such as `f/2` or `26 mm`

#### Scenario: ISO formatting
- **WHEN** ISO is known
- **THEN** it is shown as a whole number prefixed by `ISO`

#### Scenario: Partial settings
- **WHEN** only ISO is known
- **THEN** the right segment reads `ISO 100` with no separators for the omitted values

#### Scenario: No settings
- **WHEN** an item has no known camera settings
- **THEN** the viewer shows no settings segment in the status row

### Requirement: Link published geocoordinates to OpenStreetMap

The viewer SHALL show the published coordinates as the centred segment of the single status row, SHALL link them to OpenStreetMap in a new tab, and SHALL make no map request before the visitor activates the link.

#### Scenario: Location link present
- **WHEN** an item with published coordinates opens
- **THEN** the centred segment shows the coordinates and links to their OpenStreetMap location

#### Scenario: OpenStreetMap target
- **WHEN** the visitor activates the location link
- **THEN** OpenStreetMap opens in a new tab centred on the item's coordinates with a marker at that position

#### Scenario: External link isolation
- **WHEN** the location link opens
- **THEN** the new page receives no catalog referrer and cannot reach back into the viewer window

#### Scenario: No coordinates
- **WHEN** location publication is disabled or the item has no coordinates
- **THEN** the viewer shows no location segment and requests no map data

#### Scenario: No background map request
- **WHEN** the viewer displays an item with coordinates
- **THEN** no third-party map request occurs until the visitor activates the link

### Requirement: Keep viewer metadata accessible

The viewer SHALL keep the capture segment as the dialog's accessible name, SHALL expose the camera settings and location text as part of the dialog description, and SHALL give the location link an accessible name that identifies the coordinates, OpenStreetMap, and the new-tab behavior.

#### Scenario: Accessible name unchanged
- **WHEN** the viewer opens an item with camera settings
- **THEN** its accessible name remains the capture caption without the settings or filename

#### Scenario: Settings described
- **WHEN** an assistive technology reads the dialog description
- **THEN** it includes the camera settings and the location text when present

#### Scenario: Labelled location link
- **WHEN** the location link is focused
- **THEN** its accessible name identifies the coordinates, OpenStreetMap, and that it opens in a new tab

#### Scenario: Metadata does not replace the caption
- **WHEN** the settings or location segments are shown
- **THEN** the capture caption remains visible in the status row and unchanged

## ADDED Requirements

### Requirement: Present viewer status in one row

The viewer SHALL present the capture caption, published coordinates, and camera settings in one status row in the viewer caption area, with the caption at the left edge, the coordinates centred on the row, and the settings at the right edge. Missing parts SHALL be omitted without moving the remaining parts out of their assigned positions, and the row SHALL wrap or stack on narrow viewports without horizontal scrolling or overlapping text.

#### Scenario: All status parts present
- **WHEN** an item opens with a capture caption, published coordinates, and known camera settings
- **THEN** the viewer shows one status row with the caption at the left, the coordinates centred, and the settings at the right

#### Scenario: Coordinates centred relative to the row
- **WHEN** an item with published coordinates opens
- **THEN** the centre of the coordinates segment aligns with the centre of the status row, within the row's layout tolerance

#### Scenario: Coordinates absent
- **WHEN** an item has no published coordinates but has camera settings
- **THEN** the status row keeps the caption left and the settings right, with no empty centre segment

#### Scenario: Settings absent
- **WHEN** an item has published coordinates but no camera settings
- **THEN** the status row keeps the coordinates centred and the caption left, with no empty right segment

#### Scenario: Only caption present
- **WHEN** an item has neither published coordinates nor camera settings
- **THEN** the status row shows only the capture caption at the left

#### Scenario: Narrow viewport
- **WHEN** the viewer is used on a viewport too narrow to fit the status parts side by side
- **THEN** the parts wrap or stack without overlapping or causing horizontal scrolling, while the caption remains the accessible name

#### Scenario: Replaces stacked status lines
- **WHEN** any item opens in the viewer
- **THEN** the status area renders as one row rather than as separate stacked caption, settings, and location lines
