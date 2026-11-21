# Spec Delta

## ADDED Requirements

### Requirement: Provide a browsable catalog hierarchy

The runtime SHALL serve a root years index, a per-year months index, and a per-month item grid at the existing catalog routes, rendered from the published catalog as HTML pages that work without JavaScript.

#### Scenario: Root index
- **WHEN** the catalog contains items from 2023 and 2024
- **THEN** `/` lists both years with links to their year pages

#### Scenario: Year page
- **WHEN** a year page is opened
- **THEN** it lists that year's months and links to each month grid

#### Scenario: Month grid
- **WHEN** a month page is opened
- **THEN** it shows the month's thumbnails with dates, type markers, and links to originals

#### Scenario: JavaScript disabled
- **WHEN** the browser has JavaScript disabled
- **THEN** the hierarchy and month grids remain fully browsable

## MODIFIED Requirements

### Requirement: Use same-origin root-relative media URLs

The catalog SHALL reference thumbnails under `/thumbs/` and originals under `/media/` using same-origin root-relative URLs, and SHALL percent-encode path segments in JSON and rendered pages.

#### Scenario: Same-origin serving
- **WHEN** the web tree is served at `/` and the media root is mounted at `/media/`
- **THEN** generated pages load thumbnails and originals without host or CORS configuration

#### Scenario: Filename with spaces and Unicode
- **WHEN** an item is named `Familie Müller #1.jpg`
- **THEN** its thumbnail and media URLs encode the segments correctly and the link resolves

### Requirement: Keep month pages lightweight

The runtime SHALL lazy-load month thumbnails and SHALL NOT emit a separate HTML page per item.

#### Scenario: Large month
- **WHEN** a month contains roughly 500 items
- **THEN** the month page references all items but defers off-screen thumbnail loading

#### Scenario: Item interaction
- **WHEN** a visitor selects an item
- **THEN** the original opens directly from `/media/` rather than a generated detail page

### Requirement: Provide a static search and filter page

The runtime SHALL serve `search.html` that loads the JSON index and year shards in the browser and filters items by text, type, year, and month without a backend service.

#### Scenario: Global text search
- **WHEN** a visitor types part of a filename into the search page
- **THEN** matching items from the loaded shards are shown as thumbnails

#### Scenario: Filter by type
- **WHEN** the visitor selects "videos"
- **THEN** only video items remain in the result grid

#### Scenario: Search page without JavaScript
- **WHEN** JavaScript is unavailable
- **THEN** the search page still provides links to the browsable hierarchy

### Requirement: Produce deterministic static output

The system SHALL produce identical JSON, thumbnail URLs, and server-rendered catalog pages for an unchanged catalog across repeated runs, excluding fields intentionally tied to the run itself.

#### Scenario: Repeat build
- **WHEN** the generator runs twice against an unchanged media tree
- **THEN** the emitted JSON is byte-identical and the runtime renders identical catalog pages apart from permitted run metadata

#### Scenario: Changed item only
- **WHEN** one item changes in a month
- **THEN** that item's thumbnail URL changes while unchanged items keep their URLs

### Requirement: Escape untrusted names in HTML

The runtime SHALL HTML-escape item names and metadata when rendering pages and SHALL provide alternative text for thumbnails.

#### Scenario: Filename containing markup
- **WHEN** a filename contains characters such as `<script>`
- **THEN** the text is escaped and does not become executable markup

#### Scenario: Accessibility
- **WHEN** a thumbnail is rendered
- **THEN** it has alternative text derived from the item name or capture date

## REMOVED Requirements

### Requirement: Provide a browsable static hierarchy

**Reason**: The hierarchy is rendered by the Rust runtime from published JSON instead of being written as static files.

**Migration**: The existing `/`, `/{year}/`, and `/{year}/{month}/` routes are served by the runtime. Clients do not need route changes, and no-JS browsing remains available.
