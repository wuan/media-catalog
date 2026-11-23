# Spec Delta

## ADDED Requirements

### Requirement: Return to the top from each event group

The service SHALL render a back-to-top link at the end of every rendered event group: the Other bucket, each authored section, and each folder-month group on unsectioned event pages. The link SHALL follow the group's tile grid and SHALL target a top anchor on the same page. It SHALL be a plain anchor that works without JavaScript.

#### Scenario: Authored section
- **WHEN** an event page renders an authored section with items
- **THEN** a back-to-top link appears after the section's tile grid

#### Scenario: Other bucket
- **WHEN** an event page renders a non-empty Other bucket
- **THEN** a back-to-top link appears after the Other bucket's tile grid

#### Scenario: Month group on an unsectioned event
- **WHEN** an unsectioned event page renders a folder-month group
- **THEN** a back-to-top link appears after that month group's tile grid

#### Scenario: Empty event
- **WHEN** an event's range matches no items
- **THEN** the page renders its empty state without back-to-top links

#### Scenario: JavaScript disabled
- **WHEN** the browser has JavaScript disabled
- **THEN** the back-to-top link still returns the visitor to the top of the page
