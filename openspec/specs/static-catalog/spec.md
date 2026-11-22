# Static Catalog Specification

## Purpose

Defines the deployable static web output: browsable year and month pages, sharded JSON, and a client-side filter page that works alongside a same-origin media mount.

## Requirements

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
- **WHEN** a visitor selects an item on a month page and the viewer script is available
- **THEN** the item opens in the in-page viewer while the original remains reachable through its link

#### Scenario: Item interaction without script
- **WHEN** a visitor selects an item and the viewer script is unavailable
- **THEN** the original opens directly from `/media/` rather than a generated detail page

### Requirement: Emit sharded JSON catalog data

The generator SHALL emit `catalog/index.json` describing years, counts, and shard paths directly under the configured data root, and one `catalog/YYYY.json` shard per year containing item records with paths, kind, timestamps, dimensions, duration, camera model when known, thumbnail URL, and media URL.

#### Scenario: Year shards
- **WHEN** the catalog spans multiple years
- **THEN** each year has its own shard and the index points to each shard

#### Scenario: Record contents
- **WHEN** a shard is read
- **THEN** each item record provides enough data to render a thumbnail link and open the original without additional requests

#### Scenario: Model field
- **WHEN** a source item has a known camera model
- **THEN** its shard record carries the model; when no model is known, the field is omitted

#### Scenario: Data root location
- **WHEN** the generator publishes with the data root at `./data`
- **THEN** the index is written to `./data/catalog/index.json` and shards to `./data/catalog/YYYY.json`, with no `public/` segment

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

### Requirement: Present concise tile captions

The runtime SHALL render every month, event, and search grid tile with a single-line caption containing the capture time as `YY-MM-DD HH:MM` and, when known, the camera model. The caption SHALL omit the visible filename, stay on one line, and keep the item name in the thumbnail's alternative text.

#### Scenario: Caption with a model
- **WHEN** an item captured at `2024-05-01 10:00:00` has the model `iPhone 15 Pro`
- **THEN** its tile caption reads `24-05-01 10:00 iPhone 15 Pro`

#### Scenario: Caption without a model
- **WHEN** an item has no known model
- **THEN** its tile caption reads only `YY-MM-DD HH:MM`, such as `24-05-01 10:00`

#### Scenario: All grid surfaces
- **WHEN** a month page, an event page, or a search result grid is rendered
- **THEN** each tile uses the same one-line caption format

#### Scenario: Long model text
- **WHEN** the model text is wider than the tile
- **THEN** the caption remains one line and truncates instead of wrapping

#### Scenario: Accessibility preserved
- **WHEN** a tile is rendered without a visible filename caption
- **THEN** its thumbnail alternative text still derives from the item name or capture date
