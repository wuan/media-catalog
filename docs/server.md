# Runtime server

`media-catalog-server` is the runtime for a generated catalog. It renders catalog
pages from published JSON, serves the embedded search shell and browser assets,
serves catalog JSON and thumbnails, and streams originals from a read-only media
root. It does not require a pre-generated HTML tree.

## Topology

All-in-one, on a host that can read the media root:

```text
Browser
   |
   v
+--------------------------------------------------+
| media-catalog-server                             |
|  /                  -> render years and events   |
|  /events/           -> render event list         |
|  /events/{slug}/    -> render event page         |
|  /YYYY/             -> render months from JSON   |
|  /YYYY/MM/          -> render item grid          |
|  /search.html       -> embedded shell            |
|  /assets/*          -> embedded CSS/JS           |
|  /catalog/*         -> published JSON            |
|  /thumbs/*          -> data_root/thumbs/*        |
|  /media/*           -> media_root/*              |
|  /healthz           -> catalog/media readiness   |
+--------------------------+-----------------------+
                           | read-only
                           v
                    media mount (SMB/NFS/local)
```

Keep the data root and the state root on local disk. SQLite WAL is not supported
on network filesystems, and even rollback-journal mode is unreliable over SMB.
Only the original media needs to live on the NAS.

## Build and run

```sh
cargo build --release --bin media-catalog-server

export MEDIA_CATALOG_TOKEN='choose-a-long-random-value' # optional
./target/release/media-catalog-server \
  --data-root "$HOME/.media-catalog/data" \
  --media-root /Volumes/media/photos \
  --bind 127.0.0.1:8080
```

The data root may not contain a catalog yet. The service starts anyway, serves
the shell and a bootstrap page, and begins rendering catalog pages when the
generator publishes `catalog/index.json`. The media root remains required because
`/media/` depends on it. The service binds to loopback by default. Add
`--bind 0.0.0.0:8080` or a Tailscale address only when you intend to expose it.

The access token is read from `MEDIA_CATALOG_TOKEN`, never from the command line.
If the variable is unset or empty, access control is disabled.

```sh
curl -H "Authorization: Bearer $MEDIA_CATALOG_TOKEN" \
  http://127.0.0.1:8080/media/2024/05/photo.jpg
```

## Endpoints and behavior

| Route | Behavior |
| --- | --- |
| `/` | rendered years index from JSON, with an Events section above Years when configured; bootstrap page before publication |
| `/events/` | rendered event list; empty list when no events are configured; `503` before publication |
| `/events/{slug}/` | rendered event page with month sections; `404` for unknown slugs; `503` before publication |
| `/{year}/`, `/{year}/{month}/` | rendered pages; `404` for unknown years/months; `503` before publication |
| `/search.html` | embedded search shell, available without a catalog |
| `/assets/site.css`, `/assets/search.js`, `/assets/viewer.js` | embedded assets, available without a catalog |
| `/catalog/*` | published JSON from `data_root/catalog/`; `404` before publication |
| `/thumbs/*` | generated thumbnails from `data_root/thumbs/` |
| `/media/*` | read-only original stream |
| `/healthz` | `200` when catalog and media are ready, otherwise `503` with a detail line |

- Dynamic routes accept only numeric four-digit years and two-digit months, and
  are registered before the static fallback. Explicit `/events/...` routes are
  registered ahead of the numeric routes so they never fall through to year
  handling. Unknown/invalid segments return `404`, never a directory listing.
- Event pages group items by folder month, newest month and newest item first;
  month headings link to `/YYYY/MM/`. They reuse the same lazy tile markup and
  remain usable without JavaScript.
- `HEAD` returns headers without a body for catalog pages and embedded assets.
- `Range: bytes=start-end`, `bytes=start-`, and `bytes=-suffix` return `206` with
  `Content-Range`; unsatisfiable ranges return `416`.
- Originals return strong `ETag`/`Last-Modified` and honor `If-None-Match` and
  `If-Modified-Since` with `304`.
- Rendered pages carry an ETag derived from the catalog snapshot and route;
  embedded assets carry a content hash. A matching `If-None-Match` returns `304`.
- Catalog JSON and rendered pages use `no-cache`; embedded assets revalidate;
  versioned thumbnails use `public, max-age=2592000, immutable`; originals use
  `private, no-cache`.
- Path traversal, encoded traversal, NUL, backslash, and symlink escape are
  rejected.
- If the media root disappears, `/media/` returns `503`; catalog pages remain
  servable.
- The service never writes to the data or media roots and never lists
  directories.

## Catalog snapshot reload

The server builds an immutable in-memory snapshot from `catalog/index.json` and
every shard it references. On each catalog request it checks the index metadata:

- unchanged index → keep using the snapshot
- changed index → load the index and all referenced shards, then swap in the new
  snapshot
- missing or unreadable shard → keep the previous snapshot and log the reload
  failure
- duplicate event slugs, undeclared item event slugs, or declared event counts
  that disagree with shard membership → keep the previous snapshot and log the
  reload failure

Publication order makes this safe: the generator writes shards first and
`catalog/index.json` last. A visitor therefore never observes a partially
published catalog, and a running server picks up a completed update without a
restart.

Before any catalog exists, `/` serves an embedded bootstrap page explaining that
the catalog is not yet available. `/healthz` reports catalog readiness separately
from media readiness so an operator can distinguish a missing build from a
missing mount.

## Migrating from the catalog-root layout

Older deployments kept published data in `<catalog-root>/public`. There is no
legacy fallback; move it into the data root before starting this server:

```sh
mv catalog/public data
mkdir -p state
mv catalog/state.sqlite catalog/build.lock state/
```

The server no longer serves pre-generated `index.html`, year/month HTML,
`search.html`, or `assets/`. Older files may remain in the data root after an
upgrade but are ignored. Remove them manually when convenient:

```sh
find data -type f -name index.html -delete
rm -f data/search.html
rm -rf data/assets
```

Keep `data/catalog/` and `data/thumbs/`; those are the runtime data.

## Supervision

A systemd unit example lives at `docs/deploy/media-catalog-server.service`.
Keep the token in an environment file, not in the unit:

```sh
sudo install -d -m 0750 /etc/media-catalog
sudo sh -c 'printf "MEDIA_CATALOG_TOKEN=%s\n" "$(openssl rand -hex 32)" \
  > /etc/media-catalog/server.env'
sudo chmod 0600 /etc/media-catalog/server.env
```

```ini
EnvironmentFile=/etc/media-catalog/server.env
```

A launchd example lives at `docs/deploy/com.example.media-catalog-server.plist`
for macOS. Adjust the paths, install it under `~/Library/LaunchAgents/`, and load
it with `launchctl bootstrap gui/$(id -u) <plist>`. Protect the plist if it
contains a token, or use `launchctl setenv MEDIA_CATALOG_TOKEN ...` instead.

## Firewall and TLS

- Keep the default loopback bind unless LAN access is intended.
- Restrict the listening port to trusted networks with the host firewall.
- Prefer a TLS-terminating reverse proxy for remote access. It must pass
  `Authorization`, `Range`, and `If-Range`, and must not buffer `/media/`.
- The server itself speaks plain HTTP.
- Use the bearer token if the service is reachable by more than one trusted
  user, and do not log request headers.

## Browser playback versus download

Direct streaming does not transcode or remux. Behavior depends on the browser:

| Format | Typical browser behavior |
| --- | --- |
| JPEG, PNG, WebP | displayed inline |
| MP4 with H.264/AAC | usually plays inline |
| WebM with VP8/VP9 | usually plays inline |
| MOV, HEVC, MKV, AVI | often downloads or fails to play |

If HEVC/MKV playback is required, that needs a separate transcoding change with
ffmpeg, a job queue, and cached output.

## Verification

```sh
# rendered catalog pages
curl -i http://127.0.0.1:8080/
curl -i http://127.0.0.1:8080/2024/
curl -i http://127.0.0.1:8080/2024/05/
curl -i http://127.0.0.1:8080/events/
curl -i http://127.0.0.1:8080/events/alps-2024/

# embedded shell and assets
curl -i http://127.0.0.1:8080/search.html
curl -i http://127.0.0.1:8080/assets/search.js
curl -i http://127.0.0.1:8080/assets/viewer.js

# JSON
curl -i http://127.0.0.1:8080/catalog/index.json

# media headers, HEAD, and range
curl -I http://127.0.0.1:8080/media/2024/05/clip.mp4
curl -i -H 'Range: bytes=1000-1999' \
  http://127.0.0.1:8080/media/2024/05/clip.mp4

# rendered-page conditional request
etag=$(curl -sI http://127.0.0.1:8080/2024/05/ \
  | awk 'tolower($1)=="etag:" {print $2}' | tr -d '\r')
curl -i -H "If-None-Match: $etag" \
  http://127.0.0.1:8080/2024/05/

# traversal must be rejected
curl -i --path-as-is http://127.0.0.1:8080/media/../etc/passwd

# readiness
curl -i http://127.0.0.1:8080/healthz
```

Logs go to stderr in the form:

```text
GET /media/2024/05/clip.mp4 -> 206 (1000 bytes, 3 ms)
```

Credentials are never included in the log line.
