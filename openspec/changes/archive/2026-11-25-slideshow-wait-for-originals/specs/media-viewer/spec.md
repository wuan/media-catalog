# Spec Delta

## MODIFIED Requirements

### Requirement: Present images with pan and zoom

The viewer SHALL display images at their original resolution with an immediate thumbnail preview, SHALL defer requesting the original until the item has been shown for a short delay, SHALL fit each newly opened image to the viewport, and SHALL support pan and zoom gestures. While the thumbnail preview is shown, the viewer SHALL size the square 512x512 center-crop placeholder to the shorter edge of the original image's fitted box and center it, so the placeholder covers exactly the region of the original that the crop represents. The viewer SHALL keep the placeholder aligned with the current stage whenever the stage layout changes while the preview is visible. Slideshow playback is exempt from the thumbnail preview: an item already on screen remains visible while the slideshow prepares its next item.

#### Scenario: Original image
- **WHEN** an image item opens outside slideshow playback
- **THEN** the thumbnail is shown while the original loads and the original replaces it when available

#### Scenario: Original loads after a short dwell
- **WHEN** an image item with a thumbnail opens outside slideshow playback
- **THEN** the viewer shows the thumbnail immediately and only requests the original once the item has stayed open for a short delay (300 ms)

#### Scenario: Rapid browsing skips originals
- **WHEN** the visitor moves past an image item before the delay elapses, for example by holding an arrow key
- **THEN** the viewer does not request that item's original, cancels any original download still in progress for it, and does not preload its neighbours

#### Scenario: Neighbour preloading follows the original
- **WHEN** the current item's original has finished loading or failed outside slideshow playback
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

#### Scenario: Slideshow slide presentation
- **WHEN** slideshow playback opens an image item
- **THEN** no thumbnail preview is painted for that item and it becomes visible only when its original image has decoded

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

### Requirement: Show the preview until the original is ready

The viewer SHALL keep the thumbnail preview visible until the original image is decoded and ready to paint outside slideshow playback, and SHALL NOT paint a frame in which the stage shows neither the preview nor the original. During slideshow playback the viewer SHALL keep the current image visible until the next item's original is decoded.

#### Scenario: Original decoding after the preview
- **WHEN** the original image data has arrived but the image is not yet decoded outside slideshow playback
- **THEN** the viewer continues to show the thumbnail preview and the stage never shows an empty frame

#### Scenario: Original becomes paintable
- **WHEN** the original image is decoded outside slideshow playback
- **THEN** the viewer replaces the thumbnail with the original in a single paint, with no blank frame in between

#### Scenario: Navigation during decode
- **WHEN** the visitor moves to another item while the previous item's original is still decoding
- **THEN** the viewer keeps showing the new item's preview and does not install the previous item's original

#### Scenario: Original fails to decode
- **WHEN** the original cannot be loaded or decoded outside slideshow playback
- **THEN** the thumbnail preview remains visible and the viewer does not show a blank stage

#### Scenario: Slideshow transition holds the current image
- **WHEN** slideshow playback is about to show its next image before that image's original has decoded
- **THEN** the image currently on screen remains painted and the next image appears only once it is ready

## REMOVED Requirements

### Requirement: Run an automatic slideshow

**Reason**: Playback now advances only through image items, waits for each decoded original, and has bounded retry and failure handling. The fixed-timer, pause-on-video contract no longer describes the viewer's slideshow behavior.

**Migration**: The replacement requirement "Run a slideshow over decoded originals" defines the current behavior. No configuration or API migration is needed.

## ADDED Requirements

### Requirement: Run a slideshow over decoded originals

The viewer SHALL advance through the album's image items automatically every 5 seconds after an explicit start, provide a pause control, skip video items, stop at the album boundaries, and wait for the next image's original to decode before changing the displayed slide. The 5-second dwell SHALL start when a slide's original is painted.

#### Scenario: Explicit start only
- **WHEN** the viewer opens an album
- **THEN** no slideshow runs until the visitor activates the play control or the slideshow key

#### Scenario: Advancing images
- **WHEN** the slideshow is playing on an image item, that image has been painted, and 5 seconds have passed
- **THEN** the viewer shows the next image item and updates the album URL as manual navigation does

#### Scenario: Advance waits for the decoded original
- **WHEN** the dwell ends before the next image item's original has decoded
- **THEN** the current image stays on screen and the slideshow advances only once the next original is ready to paint

#### Scenario: Prefetch the next image
- **WHEN** an image slide is painted during playback
- **THEN** the viewer requests the next image item's original in the background unless the browser requests reduced data usage, so the next slide can appear without another wait

#### Scenario: Stop at the last image
- **WHEN** the slideshow advances onto the last image item in the album
- **THEN** playback stops there and leaves that image visible

#### Scenario: Restart from the last item
- **WHEN** the visitor starts the slideshow while the last album item is open
- **THEN** playback restarts from the first image item

#### Scenario: Start on a video item
- **WHEN** the visitor starts the slideshow while a video item is open
- **THEN** playback moves to the next image item, or to the first image item when no image follows

#### Scenario: Pause control
- **WHEN** the slideshow is playing and the visitor activates the pause control
- **THEN** playback stops and the viewer stays on the current item

#### Scenario: Manual interaction pauses playback
- **WHEN** the slideshow is playing and the visitor navigates manually, zooms, pans, opens the original, or the document becomes hidden
- **THEN** playback pauses and does not resume until the visitor explicitly starts it again

#### Scenario: Video items are skipped
- **WHEN** the slideshow would advance onto a video item
- **THEN** the viewer skips it, never shows it during playback, and advances to the next image item

#### Scenario: Closing stops playback
- **WHEN** the viewer closes while the slideshow is playing
- **THEN** the slideshow stops, and reopening the viewer starts paused on the requested item

#### Scenario: Fewer than two image items
- **WHEN** the album contains fewer than two image items and the visitor starts the slideshow
- **THEN** playback does not advance and no timer keeps running

### Requirement: Bound retries for slideshow originals

The viewer SHALL retry a slideshow slide's original image a limited number of times before treating the item as failed, using a timeout for each attempt so a stalled request cannot block playback indefinitely.

#### Scenario: Retry a failed original
- **WHEN** a slideshow slide's original fails to load or decode, or an attempt exceeds its timeout
- **THEN** the viewer retries the original up to 3 attempts with a timeout of 10 seconds per attempt and short backoff between attempts

#### Scenario: Slideshow fails after retries
- **WHEN** every retry attempt for a slideshow slide fails
- **THEN** playback stops and the failed item opens in the paused viewer with its thumbnail preview, caption, error note, and original link

#### Scenario: Failure keeps the stage painted
- **WHEN** a slideshow slide fails after retries while a previous image is on screen
- **THEN** the previous image remains visible until the failed item opens, and the stage never shows a blank frame
