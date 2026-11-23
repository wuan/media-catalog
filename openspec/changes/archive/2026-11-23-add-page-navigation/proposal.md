# Proposal

## Why

Month pages currently have no sequential navigation: a visitor who wants the neighbouring month must go up to the year page and back down, which breaks chronological browsing and is awkward when months span year boundaries. Event pages can also be long — a single event may hold hundreds of items — and their section navigation sits at the top, so after scrolling through a section there is no direct way back to the section links.

## What Changes

- Add previous/next month links to every month page, pointing at the nearest month page that exists in the catalog.
- Resolve those links in chronological order across the whole catalog, crossing year boundaries and skipping months without items. Omit a side when no earlier or later month page exists, so the page never links to a `404`.
- Show the month navigation at the top and repeat it at the bottom of the page, since month grids can be large.
- Add a `Back to top` link at the end of each rendered event group: the `Other` bucket, each authored section, and each folder-month group on unsectioned event pages.
- Keep all navigation as plain HTML anchors so it works without JavaScript.

## Capabilities

### New Capabilities

_None._

### Modified Capabilities

- `static-catalog`: the browsable catalog hierarchy gains sequential month navigation with defined cross-year, gap-skipping, and edge behavior.
- `event-browsing`: event page in-page navigation gains a back-to-top link after each rendered group.

## Impact

Affected code: `src/render.rs`, `templates/month.html`, `templates/event.html`, and `assets/site.css`. Render snapshots and navigation tests will change for month and event pages.

Unaffected: published JSON, routes, configuration, item ordering, event membership, and the in-page viewer.
