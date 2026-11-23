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
- **WHEN** the viewer is open, not in immersive mode, and the visitor presses Escape
- **THEN** the viewer closes

#### Scenario: Escape in immersive mode
- **WHEN** the viewer is immersive and the visitor presses Escape
- **THEN** the viewer leaves immersive mode without closing, and the next Escape closes it

### Requirement: Provide an immersive viewer layout

The viewer SHALL provide an immersive mode that fills the window, lays the stage out across the entire window so media is fitted to the maximum available area, and presents its chrome as a fading overlay above the stage without navigating away from the album page.

#### Scenario: Entering immersive mode
- **WHEN** the visitor activates the immersive toggle or presses `f`
- **THEN** the viewer expands to fill the window and the toggle reports the immersive state to assistive technology

#### Scenario: Media fills the window
- **WHEN** immersive mode is active
- **THEN** the stage spans the whole viewer and the current image or video is fitted to that stage area, whether or not chrome is visible

#### Scenario: Chrome overlays the media
- **WHEN** immersive mode is active with chrome visible
- **THEN** the top bar, controls, and caption appear above the stage without reducing the stage's size

#### Scenario: Chrome hides while idle
- **WHEN** immersive mode is active and the visitor has not moved the pointer, tapped, or focused a control for a short interval
- **THEN** the viewer hides its top, control, and caption chrome so only the media remains visible

#### Scenario: Chrome returns on interaction
- **WHEN** chrome is hidden and the visitor moves the pointer, taps the stage, uses the wheel, presses a key, or moves focus into a control
- **THEN** the viewer reveals its chrome again over the unchanged full-window stage

#### Scenario: Escape leaves immersive mode first
- **WHEN** the viewer is immersive and the visitor presses Escape
- **THEN** the viewer leaves immersive mode and remains open, and a subsequent Escape closes the viewer

#### Scenario: Leaving immersive mode
- **WHEN** the visitor deactivates the immersive toggle
- **THEN** the viewer returns to its framed layout and keeps the current item and any active slideshow state

#### Scenario: Phone-sized viewport
- **WHEN** the viewer is immersive on a viewport where the framed dialog already fills the screen
- **THEN** the immersive mode still hides chrome without changing the current item and keeps the stage at the full viewport size

### Requirement: Present images with pan and zoom

The viewer SHALL display images at their original resolution with an immediate thumbnail preview, SHALL defer requesting the original until the item has been shown for a short delay, SHALL fit each newly opened image to the viewport, and SHALL support pan and zoom gestures. While the thumbnail preview is shown, the viewer SHALL size the square 512x512 center-crop placeholder to the shorter edge of the original image's fitted box and center it, so the placeholder covers exactly the region of the original that the crop represents. The viewer SHALL keep the placeholder aligned with the current stage whenever the stage layout changes while the preview is visible.

#### Scenario: Original image
- **WHEN** an image item opens
- **THEN** the thumbnail is shown while the original loads and the original replaces it when available

#### Scenario: Original loads after a short dwell
- **WHEN** an image item with a thumbnail opens
- **THEN** the viewer shows the thumbnail immediately and only requests the original once the item has stayed open for a short delay (300 ms)

#### Scenario: Rapid browsing skips originals
- **WHEN** the visitor moves past an image item before the delay elapses, for example by holding an arrow key
- **THEN** the viewer does not request that item's original, cancels any original download still in progress for it, and does not preload its neighbours

#### Scenario: Neighbour preloading follows the original
- **WHEN** the current item's original has finished loading or failed
- **THEN** the viewer preloads the originals of the adjacent image items unless the browser requests reduced data usage

#### Scenario: Thumbnail matches the cropped region of the fitted box
- **WHEN** an image item with known upright width and height opens and its thumbnail is shown while the original loads
- **THEN** the thumbnail is shown as a square whose side equals the shorter edge of the original's fitted box, centered in the stage, so it lines up with the matching center region of the original when the original replaces it

#### Scenario: Landscape original placeholder
- **WHEN** a landscape image (width greater than height) opens with known dimensions and its thumbnail placeholder is shown
- **THEN** the placeholder width and height both equal the fitted box height (the shorter edge), leaving the left and right parts that only exist in the original empty

#### Scenario: Portrait original placeholder
- **WHEN** a portrait image (height greater than width) opens with known dimensions and its thumbnail placeholder is shown
- **THEN** the placeholder width and height both equal the fitted box width (the shorter edge), leaving the top and bottom parts that only exist in the original empty

#### Scenario: Thumbnail without known dimensions
- **WHEN** an image item without known width or height opens and its thumbnail is shown
- **THEN** the thumbnail is fitted using its own dimensions as before

#### Scenario: Placeholder tracks the final chrome layout
- **WHEN** an image item opens and filling the caption and metadata chrome changes the stage size before the thumbnail preview finishes loading
- **THEN** the placeholder is sized from the resulting stage, so its square side equals the shorter edge of the fitted box for the stage that is actually visible

#### Scenario: Placeholder follows immersive mode
- **WHEN** the visitor enters or leaves immersive mode while a thumbnail preview is visible
- **THEN** the placeholder is resized from the new full-area or framed stage before the original replaces it

#### Scenario: Placeholder follows a viewport resize
- **WHEN** the browser viewport changes size while a thumbnail preview is visible
- **THEN** the placeholder is resized from the new stage so it remains centered and equal to the shorter edge of the fitted box

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

### Requirement: Run an automatic slideshow

The viewer SHALL advance through the album automatically every 5 seconds after an explicit start, provide a pause control, stop at the album boundaries, and pause instead of advancing over video items.

#### Scenario: Explicit start only
- **WHEN** the viewer opens an album
- **THEN** no slideshow runs until the visitor activates the play control or the slideshow key

#### Scenario: Advancing images
- **WHEN** the slideshow is playing on an image item and 5 seconds pass
- **THEN** the viewer shows the next album item and updates the album URL as manual navigation does

#### Scenario: Stop at the last item
- **WHEN** the slideshow advances onto the last album item
- **THEN** playback stops there and leaves the last item visible

#### Scenario: Restart from the last item
- **WHEN** the visitor starts the slideshow while the last album item is open
- **THEN** playback restarts from the first album item

#### Scenario: Pause control
- **WHEN** the slideshow is playing and the visitor activates the pause control
- **THEN** playback stops and the viewer stays on the current item

#### Scenario: Manual interaction pauses playback
- **WHEN** the slideshow is playing and the visitor navigates manually, zooms, pans, opens the original, or the document becomes hidden
- **THEN** playback pauses and does not resume until the visitor explicitly starts it again

#### Scenario: Video pauses the slideshow
- **WHEN** the slideshow reaches a video item
- **THEN** playback pauses, the video is shown with its poster and native controls, and it does not autoplay

#### Scenario: Play continues past a video
- **WHEN** the slideshow is paused on a video item and the visitor activates play
- **THEN** the viewer advances to the next item and resumes automatic playback from there

#### Scenario: Closing stops playback
- **WHEN** the viewer closes while the slideshow is playing
- **THEN** the slideshow stops, and reopening the viewer starts paused on the requested item

#### Scenario: Single-item album
- **WHEN** the album contains only one item and the visitor starts the slideshow
- **THEN** playback does not advance and no timer keeps running

### Requirement: Preserve the no-JavaScript fallback

Catalog tiles SHALL remain regular links to original media so that albums remain usable when the viewer script is unavailable.

#### Scenario: Viewer script unavailable
- **WHEN** the browser does not run the viewer script
- **THEN** activating a tile opens the original through `/media/` as before

#### Scenario: Viewer script fails
- **WHEN** the viewer cannot initialize on a page
- **THEN** tile links continue to open originals and no album interaction is broken

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

### Requirement: Keep the viewer accessible

The viewer SHALL expose modal semantics, labelled controls, current-position information, and keyboard access for its primary actions.

#### Scenario: Dialog semantics
- **WHEN** the viewer opens
- **THEN** assistive technology identifies a modal dialog and the page behind it is not interactive

#### Scenario: Position announcement
- **WHEN** the viewer moves to a different item through manual navigation
- **THEN** the current item position and total album size are available to assistive technology

#### Scenario: Automatic progression is quiet
- **WHEN** the slideshow advances items automatically
- **THEN** the viewer does not announce each automatic move, while the pause control and current position remain available to assistive technology

#### Scenario: Labelled controls
- **WHEN** a visitor navigates the viewer controls with a keyboard or screen reader
- **THEN** previous, next, close, zoom, immersive, slideshow, and open-original actions have accessible names, and toggles report their pressed state

#### Scenario: Pause mechanism
- **WHEN** automatic playback is available
- **THEN** the visitor can pause it with a visible control that is reachable by keyboard and announced with an accessible name

#### Scenario: Focus containment
- **WHEN** the viewer is open
- **THEN** keyboard focus stays within the viewer until it closes

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
