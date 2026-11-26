# Spec Delta

## ADDED Requirements

### Requirement: Render recent items on the root page

The service SHALL render a recent-items section on the root page, after the event and year sections, using item records already loaded from the published catalog. The section SHALL contain at most 24 items ordered by capture timestamp descending, with the relative path as a deterministic descending tie-break.

#### Scenario: Newest items are selected across the catalog

- **WHEN** a client requests `/` and the catalog holds more than 24 items
- **THEN** the recent section shows the 24 items with the newest capture timestamps, not simply the items from the newest archive folder

#### Scenario: Images and videos are both eligible

- **WHEN** the newest catalog items include videos
- **THEN** the recent section includes them with the same video badge and duration marker used on month pages

#### Scenario: Small catalog

- **WHEN** the catalog holds fewer than 24 items
- **THEN** the recent section shows every item and does not pad the grid

#### Scenario: Empty catalog

- **WHEN** a catalog is published with no items
- **THEN** the recent section is omitted rather than rendered empty

#### Scenario: JavaScript unavailable

- **WHEN** JavaScript is unavailable in the browser
- **THEN** every recent tile remains a link to its original media file

### Requirement: Present recent items as a responsive tile grid

The service SHALL present recent items as catalog tiles with the standard caption, alternative text, thumbnail or placeholder, and original link, arranged in a grid that shows four rows at desktop width and reflows to fewer columns on narrower screens.

#### Scenario: Desktop width

- **WHEN** the root page renders on a wide viewport
- **THEN** the recent section presents six tile columns, so 24 items occupy four rows

#### Scenario: Narrow viewport

- **WHEN** the root page renders on a narrow viewport
- **THEN** the recent grid reduces its column count and the same items occupy more rows without clipping or hiding tiles

#### Scenario: Placeholder items

- **WHEN** a recent item has no thumbnail available
- **THEN** the section shows the same neutral placeholder used by month pages
