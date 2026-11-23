# Spec Delta

## ADDED Requirements

### Requirement: Show camera settings in the viewer

The viewer SHALL show a metadata line for the open item with aperture, exposure time, ISO, and focal length when known, using fixed units and ` · ` separators, and SHALL omit each unknown value without leaving empty separators.

#### Scenario: Complete settings
- **WHEN** an item has aperture f/1.8, exposure time 1/125 s, ISO 100, and focal length 26 mm
- **THEN** the metadata line reads `26 mm · f/1.8 · 1/125 s · ISO 100`

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
- **THEN** the metadata line reads `ISO 100` with no separators for the omitted values

#### Scenario: No settings
- **WHEN** an item has no known camera settings
- **THEN** the viewer shows no metadata line

### Requirement: Link published geocoordinates to OpenStreetMap

The viewer SHALL show a location link for each item whose coordinates are published, SHALL address that location on OpenStreetMap in a new tab, and SHALL make no map request before the visitor activates the link.

#### Scenario: Location link present
- **WHEN** an item with published coordinates opens
- **THEN** the viewer shows the coordinates and a link to their OpenStreetMap location

#### Scenario: OpenStreetMap target
- **WHEN** the visitor activates the location link
- **THEN** OpenStreetMap opens in a new tab centred on the item's coordinates with a marker at that position

#### Scenario: External link isolation
- **WHEN** the location link opens
- **THEN** the new page receives no catalog referrer and cannot reach back into the viewer window

#### Scenario: No coordinates
- **WHEN** location publication is disabled or the item has no coordinates
- **THEN** the viewer shows no location link and requests no map data

#### Scenario: No background map request
- **WHEN** the viewer displays an item with coordinates
- **THEN** no third-party map request occurs until the visitor activates the link

### Requirement: Keep viewer metadata accessible

The viewer SHALL keep the existing caption line as the dialog's accessible name, SHALL expose the camera settings and location text as part of the dialog description, and SHALL give the location link an accessible name that identifies the coordinates, OpenStreetMap, and the new-tab behavior.

#### Scenario: Accessible name unchanged
- **WHEN** the viewer opens an item with camera settings
- **THEN** its accessible name remains the capture timestamp line without the settings or filename

#### Scenario: Settings described
- **WHEN** an assistive technology reads the dialog description
- **THEN** it includes the camera settings and the location text when present

#### Scenario: Labelled location link
- **WHEN** the location link is focused
- **THEN** its accessible name identifies the coordinates, OpenStreetMap, and that it opens in a new tab

#### Scenario: Metadata does not replace the caption
- **WHEN** the metadata line is shown
- **THEN** the capture timestamp caption remains visible and unchanged
