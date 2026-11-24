# Spec Delta

## ADDED Requirements

### Requirement: Show the preview until the original is ready

The viewer SHALL keep the thumbnail preview visible until the original image is decoded and ready to paint, and SHALL NOT paint a frame in which the stage shows neither the preview nor the original.

#### Scenario: Original decoding after the preview

- **WHEN** the original image data has arrived but the image is not yet decoded
- **THEN** the viewer continues to show the thumbnail preview and the stage never shows an empty frame

#### Scenario: Original becomes paintable

- **WHEN** the original image is decoded
- **THEN** the viewer replaces the thumbnail with the original in a single paint, with no blank frame in between

#### Scenario: Navigation during decode

- **WHEN** the visitor moves to another item while the previous item's original is still decoding
- **THEN** the viewer keeps showing the new item's preview and does not install the previous item's original

#### Scenario: Original fails to decode

- **WHEN** the original cannot be loaded or decoded
- **THEN** the thumbnail preview remains visible and the viewer does not show a blank stage
