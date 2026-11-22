# Design

## Context

Event pages with authored sections render groups in this order: sections (chronological), then the "Other" bucket last. Since the "oldest first" ordering change, items within each group are sorted ascending by capture timestamp, but the Other bucket — which holds items outside all sections, typically from before the first section — still renders at the bottom. This breaks the overall chronological flow of the page. See proposal.md for motivation and the specs delta for the updated requirements.

## Goals / Non-Goals

**Goals:**
- Render the Other bucket as the first group on sectioned event pages.
- Include an "Other" entry as the first item in the section navigation when the bucket is non-empty.
- Keep unsectioned event pages (folder-month groups) unchanged.

**Non-Goals:**
- Changing item ordering within groups (already oldest-first).
- Changing the section nav behavior for events without an Other bucket.
- Changing month/year/index page ordering.

## Decisions

**Template reorder over render-layer reordering.** The change is purely presentational: move the `{% if !other_items.is_empty() %}` block in `templates/event.html` above the `{% for section in sections %}` loop, and prepend an Other link to `section_nav` in `src/render.rs`. No changes to `events.rs` (which builds the data model) are needed because `other_items` is already computed and sorted correctly.

**Section nav includes Other as first link.** The navigation should mirror the page order. When the Other bucket is non-empty, its link appears before all section links, using `id="other"` as the anchor (already present in the template).

**No data-model changes.** The `EventView` struct already separates `sections` and `other_items`; the template just needs to render them in the right order. This keeps the change minimal and avoids touching `events.rs`.

## Risks / Trade-offs

- **Snapshot tests**: Existing render snapshot tests that assert Other-after-sections ordering will need updating. The unit test `other_bucket_renders_items_outside_published_sections` currently asserts `inside.jpg` appears before `id="other"`, which will invert.
- **Viewer navigation**: The in-page viewer follows document order, so moving Other first also changes viewer order — this is the desired behavior per the spec.

## Migration Plan

No migration needed — this is a static site template change. Rebuild the catalog to regenerate event pages.

## Open Questions

None.
