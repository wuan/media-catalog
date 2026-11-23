# Spec Delta

## ADDED Requirements

### Requirement: Provide an immersive viewer layout

The viewer SHALL provide an immersive mode that fills the window and hides its chrome while idle, without navigating away from the album page.

#### Scenario: Entering immersive mode
- **WHEN** the visitor activates the immersive toggle or presses `f`
- **THEN** the viewer expands to fill the window and the toggle reports the immersive state to assistive technology

#### Scenario: Chrome hides while idle
- **WHEN** immersive mode is active and the visitor has not moved the pointer, tapped, or focused a control for a short interval
- **THEN** the viewer hides its top, control, and caption chrome so only the media remains visible

#### Scenario: Chrome returns on interaction
- **WHEN** chrome is hidden and the visitor moves the pointer, taps the stage, uses the wheel, presses a key, or moves focus into a control
- **THEN** the viewer reveals its chrome again

#### Scenario: Escape leaves immersive mode first
- **WHEN** the viewer is immersive and the visitor presses Escape
- **THEN** the viewer leaves immersive mode and remains open, and a subsequent Escape closes the viewer

#### Scenario: Leaving immersive mode
- **WHEN** the visitor deactivates the immersive toggle
- **THEN** the viewer returns to its framed layout and keeps the current item and any active slideshow state

#### Scenario: Phone-sized viewport
- **WHEN** the viewer is immersive on a viewport where the framed dialog already fills the screen
- **THEN** the immersive mode still hides chrome without changing the current item

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

## MODIFIED Requirements

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
