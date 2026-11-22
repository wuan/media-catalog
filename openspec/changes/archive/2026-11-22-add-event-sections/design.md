# Design

## Context

See `proposal.md` - Why. The current event model is a flat list of authored
events. Each event carries an inclusive `start`/`end` range and optional
description; items join by effective timestamp, and event views group matched
items by folder month. The server renders one `/events/{slug}/` page from the
published index and shards. Item membership and shards are unchanged by this
change.

A five-week trip can span only two folder months while containing hundreds of
items, so month grouping is too coarse for a useful overview. Section data must
be authored because private EXIF location fields are never extracted or
published.

## Goals / Non-Goals

**Goals:**

- Add authored bookmark sections to a single event without introducing a nested
  event hierarchy.
- Publish resolved section ranges so the server renders ranges and does not
  depend on bookmark semantics.
- Preserve byte-identical output for catalogs without sections and for events
  without sections.
- Make a later optional explicit section end a generator-only extension.
- Keep item membership, shard records, counts, and routes unchanged.

**Non-Goals:**

- Sections with their own URLs, pages, counts, or event-list entries.
- Explicit section ends in this change.
- Event-scoped month pages, pagination, or collapsible section UI.
- Automatic section discovery from folders, dates, or EXIF metadata.
- Changing event membership or the folder-authoritative year/month hierarchy.

## Decisions

### Sections are presentation, not nested events

Sections live on their parent event and have no independent event identity.
They do not appear in `/events/`, have no `/events/{slug}/` route, and do not
change an item's event slugs.

**Why:** the Portugal trip is one curated album with chapters, not a tree of
albums. Keeping sections as presentation avoids URL trees, duplicate counts,
cycle validation, and ambiguity about whether a chapter should be separately
browsable.

**Alternatives considered:** child events with a `parent` field (more powerful,
but adds hierarchy, routes, and count semantics), and inferred anchors from
range containment (no authoring, but unrelated overlapping events would become
nested by accident).

### Bookmark authoring resolves to published ranges

Authors declare `at`, not `start`/`end`. The generator resolves each section to
an inclusive `start`/`end` range before publication:

```text
section i:  start = at_i
            end   = at_(i+1) - 1 minute     (non-last)
            end   = event.end               (last)
```

The index publishes the resolved ranges. The server never sees or interprets
`at`.

**Why:** bookmark authoring matches the mental model and gives a total partition
with no gaps or overlaps. Publishing ranges keeps the server contract concrete
and lets a future explicit `end` be handled entirely by the generator.

**Alternatives considered:** publishing bookmark timestamps and deriving ranges
in the server (simpler generator, but a later explicit end would require a
server change); and requiring `start`/`end` now (more authoring, easy gaps and
overlaps).

### Inclusive minute boundaries match existing event semantics

Section ranges use inclusive minute boundaries, like events. A derived
non-last section ends at the minute before the next bookmark, so an item exactly
at the next bookmark belongs to the later section.

**Why:** item timestamps are truncated to the minute and event membership is
already inclusive. Reusing that rule avoids introducing half-open intervals that
would disagree with event membership.

**Alternatives considered:** half-open `[start, end)` intervals (cleaner
mathematically, but inconsistent with the existing event range contract and with
displayed end dates).

### Publish sections on index events, not items

`catalog/index.json` gains an optional `sections` array on each event. Each
published section has `slug`, `title`, `start`, `end`, and optional
`description`. Items do not gain a section slug, and shards are unchanged.

**Why:** section membership is derived from the item timestamp against published
ranges. Publishing per-item section slugs would duplicate data, complicate
validation, and turn a presentation concern into an item-model concern.

**Alternatives considered:** adding `sections` to shard records (more data and
more ways to disagree with the index) and publishing a separate section-to-item
mapping file (a second source of truth).

### Server renders ranges generically, including an Other bucket

The server assigns each item to the first published section whose range contains
its timestamp. Items outside all ranges are collected into an "Other" bucket,
rendered after the authored sections and omitted when empty. In bookmark mode
the bucket is normally empty.

**Why:** a generic range renderer makes the future explicit-end extension free:
gaps simply produce Other items. It also avoids coupling the server to the
bookmark derivation rule.

**Alternatives considered:** rendering only bookmark partitions now (less code,
but the future gap extension would change rendering behavior).

### Sections replace month groups only when configured

An event with sections renders section groups and in-page navigation. An event
without sections keeps the existing month-grouped page exactly as it is. A
section range that matches no items is omitted from content and navigation.

**Why:** preserves existing behavior and byte-identical output for unsectioned
events while keeping the page focused on chapters that actually contain media.
Empty-range events still show the existing summary empty state.

**Alternatives considered:** always showing empty sections (preserves authored
structure, but adds noise), and nesting month groups inside sections (useful for
very long chapters, but not needed for a five-week trip).

### Validate sections in the generator and server

The generator validates section slugs, timestamps, containment, uniqueness, and
ordering before scanning. The server validates published section ranges during
snapshot load: slug uniqueness within an event, parseable timestamps, contained
ranges, ascending order, and no overlap. An invalid snapshot keeps the previous
snapshot in service.

**Why:** consistent with existing event validation and the index-as-commit-point
model. Independent generator and server upgrades must not expose inconsistent
section pages.

**Alternatives considered:** validating only in the generator (a malformed or
hand-edited index could still reach the server).

### Future explicit end is additive

A later change can add an optional `end` to section authoring. The generator
uses it instead of the next bookmark, validates `start <= end` and
`end < next section start`, and leaves a gap when `end` is earlier than the next
bookmark. The published schema and server behavior remain unchanged.

**Why:** this is the reason for publishing resolved ranges now. The extension
touches config parsing, generator validation, docs, and tests only.

**Alternatives considered:** adding a mode flag or a separate interval section
type (more configuration surface for little benefit).

## Risks / Trade-offs

- **Page weight is unchanged** -> sections improve overview, not payload. Lazy
  loading already defers thumbnail requests; collapsible sections or event-scoped
  month pages remain a separate follow-up for very large trips.
- **Authors can create gaps only after explicit ends exist** -> gaps render as
  Other, and validation rejects overlaps, so output stays deterministic.
- **Empty sections disappear** -> document that sections with no matching items
  are omitted; this keeps the page and navigation focused.
- **Older generators reject new config** -> `events.toml` parsing uses
  `deny_unknown_fields`; deploy the generator before adding section entries.
- **Older servers ignore sections** -> the additive index field is ignored and
  those servers fall back to month grouping, which is a safe degradation.
- **Boundary mistakes move photos between chapters** -> the minute rule and
  boundary scenarios are explicit; tests cover an item exactly on a bookmark.
- **Section slugs collide with event slugs in the URL namespace** -> section
  slugs are scoped to the parent event and are used only as fragment ids, not as
  routes.

## Migration Plan

1. Ship generator and server support. Catalogs and events without sections
   remain byte-identical.
2. Add `[[events.sections]]` entries to `events.toml` and run the generator to
   publish resolved ranges.
3. Deploy the server update when sectioned pages should be served. Older servers
   safely ignore the field until then.
4. Roll back by removing section entries and rebuilding; the index omits sections
   and the previous month-grouped event page behavior returns.

## Open Questions

None.
