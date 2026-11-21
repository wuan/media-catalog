# Spec Delta

## Purpose

Defines the deployable static web output: browsable year and month pages, sharded JSON, and a client-side filter page that works alongside a same-origin media mount.

## ADDED Requirements

### Requirement: Provide a browsable static hierarchy
The generator SHALL emit a root years index, a per-year months index, and a per-month item grid, all as static HTML pages that render without JavaScript.

#### Scenario: Root index
- **WHEN** the catalog contains items from 2023 and 2024
- **THEN** `index.html` lists both years with links to their year pages

#### Scenario: Year page
- **WHEN** a year page is opened
- **THEN** it lists that year's months and links to each month grid

#### Scenario: Month grid
- **WHEN** a month page is opened
- **THEN** it shows the month's thumbnails with dates, type markers, and links to originals

#### Scenario: JavaScript disabled
- **WHEN** the browser has JavaScript disabled
- **THEN** the hierarchy and month grids remain fully browsable

### Requirement: Use same-origin root-relative media URLs
The generator SHALL reference thumbnails under `/thumbs/` and originals under `/media/` using same-origin root-relative URLs, and SHALL percent-encode path segments.

#### Scenario: Same-origin serving
- **WHEN** the web tree is served at `/` and the media root is mounted at `/media/`
- **THEN** generated pages load thumbnails and originals without host or CORS configuration

#### Scenario: Filename with spaces and Unicode
- **WHEN** an item is named `Familie Müller #1.jpg`
- **THEN** its thumbnail and media URLs encode the segments correctly and the link resolves

### Requirement: Keep month pages lightweight
The generator SHALL lazy-load month thumbnails and SHALL NOT emit a separate HTML page per item.

#### Scenario: Large month
- **WHEN** a month contains roughly 500 items
- **THEN** the month page references all items but defers off-screen thumbnail loading

#### Scenario: Item interaction
- **WHEN** a visitor selects an item
- **THEN** the original opens directly from `/media/` rather than a generated detail page

### Requirement: Emit sharded JSON catalog data
The generator SHALL emit `catalog/index.json` describing years, counts, and shard paths, and one `catalog/YYYY.json` shard per year containing item records with paths, kind, timestamps, dimensions, duration, thumbnail URL, and media URL.

#### Scenario: Year shards
- **WHEN** the catalog spans multiple years
- **THEN** each year has its own shard and the index points to each shard

#### Scenario: Record contents
- **WHEN** a shard is read
- **THEN** each item record provides enough data to render a thumbnail link and open the original without additional requests

### Requirement: Provide a static search and filter page
The generator SHALL emit `search.html` that loads the JSON index and year shards in the browser and filters items by text, type, year, and month without a backend service.

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
The generator SHALL produce identical page, JSON, and thumbnail URLs for an unchanged catalog across repeated runs, excluding fields intentionally tied to the run itself.

#### Scenario: Repeat build
- **WHEN** the generator runs twice against an unchanged media tree
- **THEN** the generated HTML and JSON are byte-identical apart from permitted run metadata

#### Scenario: Changed item only
- **WHEN** one item changes in a month
- **THEN** that item's thumbnail URL changes while unchanged items keep their URLs

### Requirement: Escape untrusted names in HTML
The generator SHALL HTML-escape item names and metadata when rendering pages and SHALL provide alternative text for thumbnails.

#### Scenario: Filename containing markup
- **WHEN** a filename contains characters such as `<script>`
- **THEN** the text is escaped and does not become executable markup

#### Scenario: Accessibility
- **WHEN** a thumbnail is rendered
- **THEN** it has alternative text derived from the item name or capture date
