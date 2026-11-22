# Proposal

## Why

On event pages with authored sections, items that fall outside every section are collected into an "Other" bucket and rendered last, after all sections. Since event pages now order items oldest-first within each group, the "Other" bucket — which typically contains items from before the first section — appears at the bottom of the page even though those items are chronologically first. This makes the overall page order inconsistent with the oldest-first principle and hides the earliest photos behind all sectioned content.

## What Changes

- The "Other" bucket on a sectioned event page SHALL render before all authored sections, so the full page reads in chronological (oldest-first) order.
- The in-page section navigation SHALL include an "Other" link as the first entry when the bucket is non-empty.
- Unsectioned event pages (grouped by folder month) SHALL be unaffected.

## Capabilities

### Modified Capabilities

- `event-browsing`: The "Render unsectioned items" and "Render event pages" requirements change: the Other bucket moves from last to first position on sectioned event pages, and the section navigation gains an Other entry.

## Impact

- `templates/event.html`: reorder the Other section before the sections loop and add it to the section nav.
- `src/render.rs`: include the Other bucket in `section_nav` as the first link when non-empty.
- Existing render tests and snapshots that assert Other-after-sections ordering will need updating.
