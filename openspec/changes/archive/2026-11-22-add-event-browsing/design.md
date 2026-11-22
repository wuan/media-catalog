# Design

## Context

The catalog is a strict `YYYY/MM` tree: the generator scans the media root, the
server loads `catalog/index.json` and its year shards into an immutable snapshot,
and pages are rendered from that snapshot. Items already carry an effective
timestamp (`captured_at`, falling back to mtime) in naive local form.

Events are a second view over the same items, so they cannot live in the folder
tree. They also must not introduce a second source of truth for the server. The
published JSON remains the only runtime input.

See `proposal.md` for motivation and `specs/event-browsing/spec.md` for the
behavior contract.

## Goals / Non-Goals

**Goals:**

- Add authored, time-ranged events without disturbing the year/month hierarchy.
- Keep the server read-only and JSON-only.
- Keep event data additive and backward/forward compatible with existing
  catalogs and servers.
- Preserve deterministic output and the index-as-commit-point publication model.
- Reuse the existing tile, escaping, lazy-loading, caching, and no-JavaScript
  behavior.

**Non-Goals:**

- Searching or filtering by event in this change.
- An event `month -> grid` substructure or pagination for very large events.
- Automatic event discovery from folder names, EXIF metadata, or dates.
- Editing events through the server.
- Changing item capture timestamps or the folder-authoritative year/month groups.

## Decisions

### Separate `events.toml` content file

The generator loads authored events from a separate TOML file, defaulting to
`./events.toml` when present and overridable with `--events`. The server never
reads it.

**Why:** event definitions are content, while `media-catalog.toml` is deployment
configuration. Keeping them separate lets authors version and change event
content without touching operational settings, and it preserves the server's
JSON-only contract.

**Alternatives considered:** an `[[events]]` table inside `media-catalog.toml`
(one file, but mixes content with deployment settings), and a hand-authored JSON
file under the data root (breaks the generator-owned publication model).

### Timestamp membership at minute precision

An item belongs to an event when its effective timestamp, truncated to the
minute, lies within the event's inclusive `start..=end` range. Event ranges use
`YYYY-MM-DD HH:MM`; item timestamps use `YYYY-MM-DD HH:MM:SS`, so truncation
makes an item at `18:30:45` match an end of `18:30`.

**Why:** the feature exists for day-level, cross-month events. Folder grouping is
only month-granular and cannot express a range like `May 28 09:00` to
`June 5 18:30`.

**Consequence:** event membership can disagree with the folder month. That is
deliberate; year/month browsing stays folder-authoritative, and event browsing
stays timestamp-authoritative.

**Alternatives considered:** folder-month overlap (consistent with existing
grouping but loses day precision) and explicit path lists (precise but manual
and not really a time range).

### Derive membership after building the catalog

`build_catalog` continues to build the year/month model exactly as it does now.
A separate pass walks the built catalog, computes event matches from each item's
effective timestamp, writes the per-item event slugs, and builds event views.
Event definitions are sorted by newest start and then slug; item slug lists are
sorted so output does not depend on config order.

**Why:** it keeps the hierarchy builder focused, makes the event layer optional,
and allows event views to be rebuilt from the same catalog model.

### Additive JSON fields without a schema-version bump

`catalog/index.json` gains an optional `events` array. Each item in a year shard
gains an optional `events` array. Both fields are omitted when empty and default
to empty when read.

**Why:** the additions do not change the meaning of existing fields. Older
servers ignore unknown fields, newer servers read old catalogs as having no
events, and catalogs without events remain byte-identical to today's output.
Bumping `schema_version` would force lockstep deployment for no compatibility
benefit.

The index remains the commit point: event metadata is in the index, item
membership is in the shards it already references, and publication order does not
change.

### Server validates event consistency during snapshot load

Snapshot loading validates that event slugs are unique, every per-item slug is
declared in the index, and each declared event count matches the shard
membership. A failed validation keeps the previous complete snapshot in service,
matching the existing partial-publication behavior.

**Why:** the server should never expose an event page backed by inconsistent
data, and the generator and server binaries can be upgraded independently.

### One event page, month-grouped

`/events/` lists events. `/events/{slug}/` renders one page with a summary and
month sections. Months and items are newest first. Month headings link to the
normal `/YYYY/MM/` page. The existing tile presentation is reused.

**Why:** curated events are expected to be trip-sized, and one page preserves the
“event as an album” experience. A generation warning at 500 items flags the
point where a future `event -> month -> grid` structure is warranted.

**Alternatives considered:** an event month-index from day one (safer for huge
events, but an extra click and a weaker event experience). This is deferred as a
separate change.

### Root page gains an Events section

`/` lists events above the existing Years list, and `/events/` is the canonical
event list. Both use the same ordering.

**Why:** events are a top-level browsing axis, not a hidden secondary page.
Keeping `/events/` gives breadcrumbs, bookmarks, and a stable parent for event
pages.

### Route precedence and catalog readiness

Explicit `/events/` and `/events/{slug}/` routes take precedence over the
numeric `/{year}` and `/{year}/{month}` routes. Before publication these routes
return `503`, consistent with the existing year/month routes; unknown slugs
return `404`.

**Why:** it keeps readiness behavior uniform and avoids accidental fallthrough
into year handling.

## Risks / Trade-offs

- **mtime fallback can skew membership** → document that items without capture
  metadata use modification time; this is already the catalog's fallback rule.
- **Very large event pages** → warn above 500 items and plan the month substructure
  as a follow-up; Option 1 is deliberately simple.
- **Event views duplicate item data in memory** → event views share the same
  catalog snapshot and are built once per snapshot load; overlapping events may
  duplicate item references, which is acceptable for curated event sizes.
- **Slug renames break URLs** → slugs are required and validated; renaming a slug
  is an intentional URL change.
- **Event config changes do not touch media** → the generator republishes JSON on
  every non-dry run, so the next run picks them up without state changes.
- **Route collision with dynamic year/month routes** → explicit static routes are
  registered and covered by tests.

## Migration Plan

1. Ship the additive JSON fields. Old servers ignore them; new servers default
   them to empty.
2. Deploy the generator with event support; catalogs without `events.toml`
   continue to produce byte-identical output.
3. Add `events.toml` when ready and run the generator to publish event metadata
   and memberships.
4. Deploy the server with event routes when event pages should become available.

Rollback is safe: removing `events.toml` and running the generator removes event
fields from the next publication, and an older server ignores them.

## Open Questions

None.
