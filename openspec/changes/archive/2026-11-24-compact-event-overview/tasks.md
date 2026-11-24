# Tasks

## 1. Compact overview data

- [x] 1.1 Remove the description from the overview link model in `src/render.rs` and drop its span from `templates/index.html` and `templates/events.html`; verify `cargo test --lib render` passes and the rendered `/events/` output no longer contains a description while the event page still does
- [x] 1.2 Add a month-level span helper used only by the overview, leaving `event_range()` serving event pages unchanged; verify unit tests cover `May 2024`, `May-Jun 2024`, `Dec 2024 - Jan 2025`, and `May 2024 - May 2026`

## 2. Secondary type treatment

- [x] 2.1 Scope a smaller muted size to `.event-list .range` and `.event-list .count` in `assets/site.css`, leaving the shared `.range` and `.count` rules unchanged; verify `/events/` shows the compact meta text below the title size and an event page header and its section headings keep their current sizes

## 3. Regression coverage

- [x] 3.1 Update `root_and_event_list_show_events_above_years` to assert the compact span and the absence of the description, and keep asserting event-before-years ordering; verify `cargo test --lib render` passes
- [x] 3.2 Add an `/events/` snapshot for a catalog with several events covering same-month, same-year, and cross-year spans; verify `cargo test --lib render` passes and the reviewed snapshot shows titles, compact spans, and counts only

## 4. Integration check

- [x] 4.1 Run `cargo test` and confirm no render, server, or snapshot regressions
- [x] 4.2 Run `openspec validate compact-event-overview --strict` and confirm the change and delta spec report valid

## Workflow follow-up

- Archive the change after review, then verify the archived `event-browsing` spec carries the compact overview requirement and scenarios.
