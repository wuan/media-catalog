# Spec Delta

## MODIFIED Requirements

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
