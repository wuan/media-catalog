# Spec Delta

## MODIFIED Requirements

### Requirement: Link published geocoordinates to OpenStreetMap

The viewer SHALL show the published coordinates as the centred segment of the single status row and SHALL link them to OpenStreetMap in a new tab. When the deployment enables the location preview, the viewer SHALL reveal a map preview on hover or keyboard focus of that link, SHALL make no map request before that intent, and SHALL make at most one preview request per item. When the preview is disabled, no map request SHALL occur before the visitor activates the link.

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

#### Scenario: Pointer reveals the preview
- **WHEN** the location preview is enabled and the visitor points at the location link
- **THEN** a map preview appears without navigating away or opening a new tab

#### Scenario: Keyboard focus reveals the preview
- **WHEN** the location preview is enabled and the location link receives keyboard focus
- **THEN** the same map preview appears

#### Scenario: No background map request
- **WHEN** the viewer displays an item with coordinates and the preview is enabled or disabled
- **THEN** no third-party map request occurs until the visitor hovers, focuses, or activates the location link

#### Scenario: One request per item
- **WHEN** the visitor reveals the preview, leaves it, and reveals it again for the same item
- **THEN** the viewer reuses the preview without a second map request

#### Scenario: Preview disabled
- **WHEN** the deployment does not enable the location preview and the visitor hovers or focuses the location link
- **THEN** no third-party map request occurs until the visitor activates the link

#### Scenario: Preview unavailable
- **WHEN** the preview cannot load because the map service is unreachable or blocked
- **THEN** the coordinates and the OpenStreetMap link remain visible and usable without a layout jump

#### Scenario: Coarse pointer
- **WHEN** the visitor uses a touch input that cannot hover
- **THEN** the viewer offers no hover preview and activating the link opens OpenStreetMap in a new tab

### Requirement: Keep viewer metadata accessible

The viewer SHALL keep the capture segment as the dialog's accessible name, SHALL expose the camera settings and location text as part of the dialog description, and SHALL give the location link an accessible name that identifies the coordinates, OpenStreetMap, and the new-tab behavior. When the location preview is shown, its frame SHALL carry an accessible name identifying the coordinates and SHALL stay outside the viewer's tab order.

#### Scenario: Accessible name unchanged
- **WHEN** the viewer opens an item with camera settings
- **THEN** its accessible name remains the capture caption without the settings or filename

#### Scenario: Settings described
- **WHEN** an assistive technology reads the dialog description
- **THEN** it includes the camera settings and the location text when present

#### Scenario: Labelled location link
- **WHEN** the location link is focused
- **THEN** its accessible name identifies the coordinates, OpenStreetMap, and that it opens in a new tab

#### Scenario: Labelled preview
- **WHEN** the location preview is shown
- **THEN** its frame carries an accessible name that identifies the coordinates

#### Scenario: Preview stays out of the tab order
- **WHEN** the visitor presses Tab while the preview is shown
- **THEN** focus moves through the viewer's controls without entering the preview frame

#### Scenario: Preview is dismissable
- **WHEN** the visitor moves the pointer away, moves focus away, or presses Escape
- **THEN** the preview hides while the coordinates and link remain in place

#### Scenario: Metadata does not replace the caption
- **WHEN** the settings or location segments are shown
- **THEN** the capture caption remains visible in the status row and unchanged
