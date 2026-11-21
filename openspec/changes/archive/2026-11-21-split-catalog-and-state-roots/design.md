# Design

## Context

See `proposal.md` for motivation and `specs/` for the behavior contract.

Today `Config` resolves one `catalog_root` into two derived paths: `public_root = catalog_root/public` and `state_db = catalog_root/state.sqlite`. The server accepts the same catalog root and derives `public_root` from it. `BuildLock` already places `build.lock` beside `state.sqlite`, and publication/prune helpers already take the served tree root, so the change is mainly a configuration and path re-rooting exercise.

Constraints that shape the approach:

- SQLite state must remain on local storage outside the served tree, and the lock must stay beside the database.
- The runtime server does not open the state database; it only reads JSON, thumbnails, and originals.
- The existing `/catalog/*`, `/thumbs/*`, and `/media/*` routes and the JSON schema stay unchanged.
- The `server-render-catalog` change is still in flight and touches the same `static-catalog` and `media-server` requirements.

## Goals / Non-Goals

**Goals:**

- Make each storage root independently configurable and independently owned.
- Remove the `public/` wrapper and make the data root the served tree.
- Make root overlap impossible to configure accidentally.
- Keep the migration to a clean re-rooting with no compatibility shim.

**Non-Goals:**

- Changing catalog URL routes, JSON fields, shard naming, or thumbnail naming.
- Introducing a runtime database or making the server read state.
- Moving or modifying the media root.
- Supporting a legacy `catalog-root/public` layout in parallel.

## Decisions

### Use `--data-root` and `--state-root`

Replace `--catalog-root` with two flags on the generator and one on the server:

```text
media-catalog        --media-root <path> --data-root <path> --state-root <path>
media-catalog-server --media-root <path> --data-root <path>
```

Internally, `Config` gains `data_root` and `state_root`; `state_db` becomes `state_root/state.sqlite`, and `public_root` is renamed to `data_root` throughout the generator and server.

Rationale: each flag has one owner and one lifecycle. The server no longer needs a root it never writes to. Alternative considered: keep `--catalog-root` and add `--state-root`, deriving `data_root = catalog_root/public`. Rejected because it preserves the misleading wrapper and keeps the server coupled to a generator concern.

### Make the data root the published tree

Drop the intermediate `public/` directory. Publication writes directly to `<data-root>/catalog/` and thumbnails to `<data-root>/thumbs/`.

Rationale: `public` only existed to keep state outside the served tree. With a separate state root there is nothing to separate it from. Alternative considered: keep `public/` inside the data root. Rejected because it adds a level with no meaning and continues to imply static-site deployment.

### Keep the state file name fixed

`state.sqlite` and `build.lock` are fixed names under `--state-root`; no `--state-db` flag is added.

Rationale: one state root maps to one build history, and the lock already derives its path from the database's parent. A separate database path would allow one lock to guard a different database than the one opened. Alternative considered: accept a full `--state-db` path. Rejected as unnecessary surface for the current use case.

### Validate all root relationships in one place

`Config::validate` rejects equal or nested roots for every pair, and the server keeps its own data/media separation check. Use the existing lexical-normalized paths and `path_contains` helper rather than filesystem canonicalization, so validation works before directories exist.

Rationale: a single validation point keeps generator and server errors consistent and prevents publishing into the media tree or serving state. Alternative considered: rely on deployment discipline. Rejected because a mistyped flag could write into a read-only NAS mount or publish state to the web.

### No compatibility shim

`--catalog-root` is removed rather than aliased, and neither binary falls back to `public/`. The TOML parser's `deny_unknown_fields` already gives a clear error for old configuration keys.

Rationale: a fallback would silently preserve the confusing layout and make root ambiguity harder to debug. Alternative considered: read `public/` when `data_root/catalog` is absent. Rejected because it hides misconfiguration and keeps two supported layouts.

## Risks / Trade-offs

- **Existing deployments break immediately** → provide explicit move commands and update systemd/launchd examples in the same change; call out the breaking change in the commit message.
- **Users can still swap `--data-root` and `--state-root`**, which validation cannot detect → log both roots at startup and document the expected tree; keep the state-file deny-list in the static handler as defense in depth.
- **Tests encode `public/` in fixtures and path assertions** → update the shared fixture to expose `data` and `state` paths, then mechanically replace assertions.
- **Deleting the state root forces full reprocessing** → document that published data remains valid; only rebuild time and probe/thumbnail work are lost.
- **Spec merge conflict with `server-render-catalog`** → archive and sync that change before archiving this one, so this change's modified requirements apply to the post-render spec state.

## Migration Plan

For an existing deployment rooted at `catalog/`:

```sh
mv catalog/public data
mkdir -p state
mv catalog/state.sqlite catalog/build.lock state/ 2>/dev/null || true
```

Then update commands and configuration:

```sh
media-catalog        --media-root ./media --data-root ./data --state-root ./state
media-catalog-server --media-root ./media --data-root ./data
```

In TOML, replace `catalog_root = "./catalog"` with `data_root = "./data"` and `state_root = "./state"`. Update systemd `ExecStart` and `ReadOnlyPaths` to point at the data root; keep the state root off the server's read-only path list because the server does not receive it.

Rollback requires the previous binary and moving the directories back. The JSON and thumbnail contents themselves are unchanged.
