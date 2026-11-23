# Spec Delta

## ADDED Requirements

### Requirement: Present viewer controls as icon-only controls

Every viewer control SHALL present a recognizable icon instead of a visible text label, while keeping the control's action recoverable through an accessible name and a hover/focus label. Every icon-only control SHALL keep the existing keyboard reachability, focus visibility, disabled appearance, and minimum touch target.

#### Scenario: Text replaced by icons
- **WHEN** the viewer opens
- **THEN** the close, previous, next, play/pause, zoom-out, zoom-in, reset-zoom, immersive, and open-original controls show icons and no visible text labels

#### Scenario: Accessible names preserved
- **WHEN** assistive technology or keyboard focus reaches an icon-only control
- **THEN** the control still exposes the same action name it had as a text control

#### Scenario: Labels available to sighted users
- **WHEN** a pointer hovers over an icon-only control or keyboard focus lands on it
- **THEN** the control shows its action label as a tooltip without moving the surrounding controls

#### Scenario: Icons are decorative to assistive technology
- **WHEN** assistive technology reads an icon-only control
- **THEN** it announces the control's action name without announcing the icon graphic itself

#### Scenario: Touch target preserved
- **WHEN** the viewer is used on a touch-sized viewport
- **THEN** each icon-only control keeps at least the existing minimum control size and remains tappable

#### Scenario: Disabled icon controls
- **WHEN** previous or next navigation is unavailable at an album boundary, or image-only controls are shown for a video
- **THEN** the affected icon control looks unavailable and cannot be activated, while its icon remains recognizable

### Requirement: Reflect control state in viewer icons

Viewer controls that represent a toggle SHALL swap their icon when their state changes, and SHALL keep the icon consistent with the control's updated accessible name and pressed state.

#### Scenario: Play becomes pause
- **WHEN** the visitor starts the slideshow
- **THEN** the slideshow control shows a pause icon, remains pressed, and is named for pausing the slideshow

#### Scenario: Pause becomes play
- **WHEN** the visitor pauses the slideshow
- **THEN** the slideshow control shows a play icon, reports the unpressed state, and is named for playing the slideshow

#### Scenario: Enter immersive mode
- **WHEN** the visitor enters immersive mode
- **THEN** the immersive control shows an exit/restore icon, remains pressed, and is named for leaving immersive mode

#### Scenario: Leave immersive mode
- **WHEN** the visitor leaves immersive mode
- **THEN** the immersive control shows the enter icon, reports the unpressed state, and is named for entering immersive mode
