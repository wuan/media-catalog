# Spec Delta

## MODIFIED Requirements

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
