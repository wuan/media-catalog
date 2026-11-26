# Tasks

## 1. Catalog model order

- [x] 1.1 Change `build_catalog` in `src/model.rs` to keep years newest first, iterate months ascending by number, and sort items ascending by `captured_at` with the ascending path tie-break; verify `cargo test model::tests` passes.
- [x] 1.2 Replace the newest-first model tests with hybrid expectations and add cases for newest-first years, ascending sparse months, oldest-first items, and timestamp tie-breaks; verify the new tests fail against the old model and pass after 1.1.

## 2. Published JSON and search

- [x] 2.1 Update the JSON writer snapshots so `index.json` years stay newest first and each year's `months` array ascends, while shard items group by ascending month and list oldest first; verify `cargo test json::tests` and the updated snapshots pass.
- [x] 2.2 Update search and pipeline assertions that depend on shard order, and add a check that search results follow the archive order (newest year, ascending month, oldest item); verify `cargo test --test search --test pipeline`.
- [x] 2.3 Document the published JSON order in `README.md`, including that years remain newest first while months and items ascend; verify the documented order matches the JSON fixtures and snapshots.

## 3. Rendered archive pages

- [x] 3.1 Update render tests and snapshots so year pages list months `01` through `12` and month pages show items oldest first; verify `cargo test render::tests` and the updated snapshots pass.
- [x] 3.2 Add a render test with multiple months and multiple items per month that asserts year page month order and month page item order together; verify the test exercises both orderings and passes.
- [x] 3.3 Update the catalog page behavior description in `docs/server.md` to state years newest first, months ascending, and month items oldest first; verify the text matches the rendered output.

## 4. Event pages and end-to-end behavior

- [x] 4.1 Confirm `build_event_views` and the event renderer keep their oldest-first item and month-group order after the catalog model change, updating event tests or snapshots only if the hybrid catalog order leaks into event pages; verify `cargo test events::tests render::tests`.
- [x] 4.2 Update end-to-end tests that assert month lists, tile order, or JSON order, and add coverage that an event page still reads chronologically while archive pages use the hybrid order; verify `cargo test --test end_to_end`.

## 5. Integration verification

- [x] 5.1 Run `cargo test` and `openspec validate "order-catalog-chronologically"`; verify both succeed with no new warnings beyond the pre-existing long requirement warning.
- [x] 5.2 Build a fixture catalog spanning multiple years, months, and items per month, then inspect `/`, `/YYYY/`, `/YYYY/MM/`, `catalog/index.json`, and the shard; verify years are newest first, months ascend, items are oldest first, and event pages remain chronological.

## Workflow follow-up

- Archive the change after review and after the implementation tasks are complete.
- Republish the sample catalog so deployed pages pick up the new order.
