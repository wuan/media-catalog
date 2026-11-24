# Proposal

## Why

The event overview is the first thing a visitor meets, but it reads heavier than
the year and month grids it sits above. Each card carries a minute-precision
range such as `2024-05-28 09:00 to 2024-06-05 18:30`, and `/events/` adds the
full description inside a card that is only 11rem wide. The grid turns tall and
ragged, and scanning several trips means reading timestamps twice over. The home
page already omits descriptions, so the same catalog renders two different
overview cards depending on where you look.

## What Changes

- Render the event overview as compact cards: title, a month-level span, and the
  item count. Stop rendering the description in the overview on both `/` and
  `/events/`.
- Format the span from the event's inclusive start and end months:
  - same month and year: `May 2024`
  - same year, different months: `May-Jun 2024`
  - different years: `Dec 2024 - Jan 2025`
  - same month, different years: `May 2024 - May 2026`
  Days and times are intentionally dropped; short events read as their month.
- Present the span and count as secondary text at a smaller size than the title,
  scoped to the overview list so event pages and section headings keep their
  current type treatment.
- Leave event pages unchanged: they keep full timestamps, descriptions, sections,
  and month grouping.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `event-browsing`: the requirement to serve an event list at `/events/` and an
  Events section at `/` now defines the overview entry's content and span format.

## Impact

- `src/render.rs`: `EventLink` no longer carries a description; the overview uses
  a new month-level span formatter while `event_range()` keeps serving event
  pages.
- `templates/events.html`, `templates/index.html`: drop the description span and
  use the compact span.
- `assets/site.css`: smaller, muted type for span and count inside `.event-list`
  only.
- `src/render.rs` tests and a new `/events/` snapshot.
- `openspec/specs/event-browsing/spec.md` after archive.
