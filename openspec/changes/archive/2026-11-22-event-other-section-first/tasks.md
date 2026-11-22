# Tasks

## 1. Template change

- [x] 1.1 Move the Other bucket block above the sections loop in `templates/event.html` so it renders first on sectioned event pages; verify with `cargo test render::` that the HTML output shows `id="other"` before `id="lisbon"` in a sectioned event with unsectioned items.

## 2. Section navigation

- [x] 2.1 In `src/render.rs`, prepend an "Other" `SectionLink` to `section_nav` when `other_items` is non-empty; verify with `cargo test render::sectioned_event_page_navigates` that the nav contains `href="#other"` before `href="#lisbon"`.

## 3. Tests and snapshots

- [x] 3.1 Update `other_bucket_renders_items_outside_published_sections` in `src/render.rs` so the assertion reflects Other-before-sections ordering (e.g., `inside.jpg` now appears after `id="other"`); verify with `cargo test render::other_bucket`.
- [x] 3.2 Update any insta snapshot tests that capture sectioned event pages with an Other bucket; verify with `cargo test` and `cargo insta review` if needed.
- [x] 3.3 Run the full test suite with `cargo test` and confirm all tests pass.
