# Media Viewer Specification

## Purpose

Defines an in-page media viewer that treats the current catalog page as an album, supports keyboard, touch, and pointer navigation, and provides inline image pan-zoom and video playback without leaving the page.

## Requirements

### Requirement: Derive the album from the current page

The viewer SHALL treat the catalog tiles rendered on the current page, in document order, as the album and SHALL restrict navigation to that album.

#### Scenario: Month page album
- **WHEN** a visitor opens a tile on `/2024/05/`
- **THEN** previous and next navigation move through that month's items only

#### Scenario: Event page album
- **WHEN** a visitor opens a tile on a sectioned event page
- **THEN** navigation moves through every item on that event page, crossing section boundaries and including the Other bucket when present

#### Scenario: Search result album
- **WHEN** a visitor opens a tile on a filtered search page
- **THEN** previous and next navigation move through only the currently visible result tiles

#### Scenario: Filtered item disappears
- **WHEN** filter changes while the viewer is open and the current item no longer matches
- **THEN** the viewer closes and the updated result grid remains visible

### Requirement: Open and close the viewer in place

The viewer SHALL open from a tile activation without navigating to a separate item page, and SHALL return focus to the originating tile when closed.

#### Scenario: Plain tile activation
- **WHEN** a visitor activates a tile with a plain click or keyboard activation
- **THEN** the viewer opens on that item without leaving the album page

#### Scenario: Modifier activation
- **WHEN** a visitor activates a tile with a modifier key, middle click, or another new-tab gesture
- **THEN** the original media opens through the existing tile link

#### Scenario: Closing
- **WHEN** the viewer is open and the visitor uses the close control, activates the backdrop, or presses Escape
- **THEN** the viewer closes and focus returns to the tile that opened it

#### Scenario: Deep-linked opening
- **WHEN** the viewer opens directly from a deep link rather than a tile
- **THEN** focus enters the viewer controls and closing returns focus sensibly without requiring an originating tile

### Requirement: Address the open item

The viewer SHALL reflect the open item in the album route as an `item` query parameter containing the item's relative path, and SHALL enter or leave the viewer from that URL state.

#### Scenario: Open item URL
- **WHEN** a visitor navigates to another item in the viewer
- **THEN** the address reflects that item's relative path without adding a browser history entry

#### Scenario: Refresh and share
- **WHEN** an album URL carrying a valid `item` parameter is opened or refreshed
- **THEN** the viewer opens on the matching tile after the page or search results are ready

#### Scenario: Search filters preserved
- **WHEN** a search deep link opens an item
- **THEN** the existing filter parameters remain in the address alongside the item parameter

#### Scenario: Stale item
- **WHEN** the `item` parameter names an item not present in the current album
- **THEN** no viewer opens and the album page remains usable

### Requirement: Navigate with keys, controls, and swipes

The viewer SHALL provide previous and next navigation through keyboard, on-screen controls, and horizontal touch gestures, and SHALL stop at album boundaries.

#### Scenario: Arrow keys
- **WHEN** the viewer has focus and the visitor presses Left or Right
- **THEN** the viewer shows the previous or next item in the album

#### Scenario: Album boundaries
- **WHEN** the viewer is on the first or last item of the album
- **THEN** the corresponding navigation direction does nothing and its control appears unavailable

#### Scenario: Horizontal swipe at fit
- **WHEN** the current image is at fit scale and the visitor performs a horizontal swipe
- **THEN** the viewer advances to the previous or next item according to the swipe direction

#### Scenario: Escape closes
- **WHEN** the visitor presses Escape while the viewer is open
- **THEN** the viewer closes

### Requirement: Present images with pan and zoom

The viewer SHALL display images at their original resolution with an immediate thumbnail preview, SHALL fit each newly opened image to the viewport, and SHALL support pan and zoom gestures. While the thumbnail preview is shown, the viewer SHALL size it so the square 512x512 center-crop placeholder fills the longer edge of the original image's fitted box, making the placeholder occupy the same display area the original will cover along its longer dimension.

#### Scenario: Original image
- **WHEN** an image item opens
- **THEN** the thumbnail is shown while the original loads and the original replaces it when available

#### Scenario: Thumbnail fills the longer edge of the fitted box
- **WHEN** an image item with known upright width and height opens and its thumbnail is shown while the original loads
- **THEN** the thumbnail is scaled so its longer edge matches the longer edge of the original's fitted box in the stage, cover-filling that dimension so swapping in the original causes no layout shift along the longer axis

#### Scenario: Thumbnail matches the original's display box
- **WHEN** an image item with known upright width and height opens and its thumbnail is shown while the original loads
- **THEN** the thumbnail occupies the same fitted size and position the original will occupy, cover-scaled from its center 512x512 crop, so swapping in the original causes no layout shift

#### Scenario: Landscape original placeholder
- **WHEN** a landscape image (width greater than height) opens with known dimensions and its thumbnail placeholder is shown
- **THEN** the placeholder width equals the fitted box width (the longer edge) and its height is scaled proportionally from the square crop

#### Scenario: Portrait original placeholder
- **WHEN** a portrait image (height greater than width) opens with known dimensions and its thumbnail placeholder is shown
- **THEN** the placeholder height equals the fitted box height (the longer edge) and its width is scaled proportionally from the square crop

#### Scenario: Thumbnail without known dimensions
- **WHEN** an image item without known width or height opens and its thumbnail is shown
- **THEN** the thumbnail is fitted using its own dimensions as before

#### Scenario: Zoom reset
- **WHEN** the viewer moves to a different item
- **THEN** the new image opens at fit scale regardless of the previous item's zoom or pan

#### Scenario: Pinch and wheel zoom
- **WHEN** the visitor pinches the image or uses the mouse wheel
- **THEN** the image zooms around the gesture point within the supported zoom range

#### Scenario: Double-tap zoom
- **WHEN** the visitor double-taps or double-clicks the image
- **THEN** the image toggles between fit scale and a zoomed scale centred on the gesture point

#### Scenario: Keyboard zoom
- **WHEN** the visitor presses the zoom-in, zoom-out, or reset shortcut while an image is open
- **THEN** the image zooms in, zooms out, or returns to fit scale

#### Scenario: Pan while zoomed
- **WHEN** the image is zoomed and the visitor drags or swipes
- **THEN** the image pans instead of navigating, and panning is clamped so the image cannot be moved out of view

#### Scenario: Navigation while zoomed
- **WHEN** the image is zoomed and the visitor presses Left or Right
- **THEN** the viewer navigates to the adjacent item

### Requirement: Play videos inline

The viewer SHALL play video items inline with native playback controls, the thumbnail as a poster, and metadata-only preloading, and SHALL stop playback when leaving the item.

#### Scenario: Video opens
- **WHEN** a video item opens
- **THEN** the viewer shows the poster image with native controls and does not autoplay

#### Scenario: Video focus owns arrow keys
- **WHEN** the video element owns focus and the visitor presses Left or Right
- **THEN** the native player seeks instead of navigating the album

#### Scenario: Navigation pauses playback
- **WHEN** the visitor navigates away from a playing video
- **THEN** playback stops and later visiting the item starts paused

#### Scenario: Video preview cost
- **WHEN** a video item opens or neighbours are prepared
- **THEN** only metadata is loaded for the video and the full file is not fetched until playback requires it

#### Scenario: Playback unsupported
- **WHEN** a video cannot be played by the browser
- **THEN** the viewer keeps the item caption and a direct link to the original available

### Requirement: Preserve the no-JavaScript fallback

Catalog tiles SHALL remain regular links to original media so that albums remain usable when the viewer script is unavailable.

#### Scenario: Viewer script unavailable
- **WHEN** the browser does not run the viewer script
- **THEN** activating a tile opens the original through `/media/` as before

#### Scenario: Viewer script fails
- **WHEN** the viewer cannot initialize on a page
- **THEN** tile links continue to open originals and no album interaction is broken

### Requirement: Keep the viewer accessible

The viewer SHALL expose modal semantics, labelled controls, current-position information, and keyboard access for its primary actions.

#### Scenario: Dialog semantics
- **WHEN** the viewer opens
- **THEN** assistive technology identifies a modal dialog and the page behind it is not interactive

#### Scenario: Position announcement
- **WHEN** the viewer moves to a different item
- **THEN** the current item position and total album size are available to assistive technology

#### Scenario: Labelled controls
- **WHEN** a visitor navigates the viewer controls with a keyboard or screen reader
- **THEN** previous, next, close, zoom, and open-original actions have accessible names

#### Scenario: Focus containment
- **WHEN** the viewer is open
- **THEN** keyboard focus stays within the viewer until it closes
