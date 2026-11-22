# Spec Delta

## MODIFIED Requirements

### Requirement: Present images with pan and zoom

The viewer SHALL display images at their original resolution with an immediate thumbnail preview, SHALL fit each newly opened image to the viewport, and SHALL support pan and zoom gestures. While the thumbnail preview is shown, the viewer SHALL size it to the fitted box of the original image so the original replaces it without a layout shift.

#### Scenario: Original image
- **WHEN** an image item opens
- **THEN** the thumbnail is shown while the original loads and the original replaces it when available

#### Scenario: Thumbnail matches the original's display box
- **WHEN** an image item with known upright width and height opens and its thumbnail is shown while the original loads
- **THEN** the thumbnail occupies the same fitted size and position the original will occupy, cover-scaled from its center 512x512 crop, so swapping in the original causes no layout shift

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
