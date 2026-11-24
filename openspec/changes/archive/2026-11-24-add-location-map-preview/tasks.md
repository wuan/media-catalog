# Tasks

## 1. Configuration setting

- [x] 1.1 Add `--location-preview` and the `location_preview` TOML key to the CLI, file schema, and resolved config, following the existing `include_location` shape; verify `cargo test config` covers flag, TOML, flag-wins, and default-off.
- [x] 1.2 Validate that preview requires published location, exiting non-zero with an error naming `location_preview` and `include_location`; verify a test asserts the message and that no output is written.
- [x] 1.3 Reject non-boolean `location_preview` values with an error naming the key; verify the test mirrors the existing `include_location` non-boolean case.
- [x] 1.4 Accept and ignore the key in the server binary; verify the server config test parses the file with `location_preview = true` and starts normally.
- [x] 1.5 Update `README.md` with the flag-table row, the TOML example, and the ignored-keys list; verify `cargo test` still parses the documented example.

## 2. Static output advertising

- [x] 2.1 Emit `location_preview: true` in `catalog/index.json` when enabled and omit it when disabled; verify `json.rs` tests assert presence when on and absence when off.
- [x] 2.2 Extend the JSON privacy test so the enabled case expects the preview field while the disabled case still rejects `location`, `latitude`, and `longitude`; verify both directions fail if the emission logic is inverted.
- [x] 2.3 Pass the resolved setting into month and event rendering and emit `data-location-preview="true"` on the body only when enabled; verify render tests assert the attribute per mode and existing snapshots stay unchanged with the default off.
- [x] 2.4 Propagate the index value onto the page body in `assets/search.js` before results render; verify the viewer browser test observes the attribute on the search page after the index loads.
- [x] 2.5 Document the index field, the body attribute, and the older-output fallback in `README.md`; verify the documented shape matches a generated fixture.

## 3. Viewer preview

- [x] 3.1 Read the propagated setting at hover/focus intent and only offer a preview when the item has finite published coordinates; verify no preview appears for items without coordinates or when the flag is absent.
- [x] 3.2 Build the popover frame against the official OpenStreetMap embed with a bbox around the point, a marker, `referrerpolicy="no-referrer"`, `loading="lazy"`, a coordinate-naming `title`, and `tabindex="-1"`; verify the browser test asserts the request URL contains `export/embed.html`, the bbox, and the marker.
- [x] 3.3 Add the intent delay, focus trigger, cancellation, and per-item reuse; verify zero map requests before intent, exactly one after hover/focus, and none on a second reveal of the same item.
- [x] 3.4 Place the popover above the link, clamp it inside the dialog, flip it when space is short, and hide it on leave, blur, Escape, item change, and viewer close; verify the browser test asserts visibility, dismissal, and in-bounds placement, including narrow viewports and immersive mode.
- [x] 3.5 Keep the coordinates readable when the frame cannot load, without a layout jump; verify the browser test blocks the embed host and asserts the fallback text and the intact link remain.
- [x] 3.6 Keep the preview out of the tab cycle and accessible by name; verify the browser test presses Tab and asserts focus stays in the viewer controls, and that the dialog description and location link label are unchanged.
- [x] 3.7 Leave touch input without a preview and keep the link's new-tab behavior; verify a touch-enabled browser context of the viewer test asserts no preview and the unchanged link target.
- [x] 3.8 Update the `README.md` viewer Location bullet and correct the stale privacy paragraph that still claims geocoordinates are never extracted; verify the rendered sections describe the opt-in flag and the on-intent request accurately.

## 4. Integration verification

- [x] 4.1 Run `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test`; verify all pass.
- [x] 4.2 Run `npm run test:viewer`; verify every browser assertion passes in both the default-off and preview-enabled fixtures.
- [x] 4.3 Run `openspec validate add-location-map-preview --strict`; verify the change is valid.
- [x] 4.4 Build a fixture with both settings, hover and focus the coordinates, then rebuild without the preview flag; verify the preview appears only in the enabled build and the disabled build makes no map request.

## Workflow follow-up

- Archive the change after review requirements are satisfied.
- Note the new opt-in flag and the preview request behavior in the release notes.
