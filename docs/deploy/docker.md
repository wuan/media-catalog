# Docker deployment (server only)

The container runs `media-catalog-server` and nothing else. It does **not**
include the `media-catalog` importer, `ffmpeg`, `ffprobe`, or a scheduler. Build
the catalog with the host importer described in [`docs/server.md`](../server.md)
and let the container serve it read-only.

## Image contents

The runtime layer contains:

- `media-catalog-server`
- `docker-entrypoint.sh`, which starts the server
- `healthcheck.sh`, which calls `/healthz` with the bearer token when one is set
- `curl` and CA certificates for the health check

The image runs as the numeric non-root user `10001:10001` and has no writable
system directories. Compose adds a read-only root filesystem, drops all
capabilities, and mounts only `/tmp` as a small tmpfs.

## Build the image

From the repository root:

```sh
docker build -t media-catalog-server:local .
```

The build context excludes `.git`, `target/`, `media/`, `data/`, `state/`,
`public/`, and local deployment artifacts (`.env`, Compose overrides), so the
archive and catalog data are never sent to the builder.

Verify the image directly:

```sh
docker run --rm media-catalog-server:local --help
docker run --rm --entrypoint sh media-catalog-server:local -c 'id -u'   # 10001
docker run --rm --entrypoint sh media-catalog-server:local -c \
  'for tool in media-catalog ffmpeg ffprobe; do ! command -v "$tool"; done'
```

`media-catalog-server --help` must print usage and exit successfully. The last
command exits successfully only when the importer binary and both media tools
are absent.

## Entrypoint

With no arguments the container runs:

```sh
media-catalog-server --config /etc/media-catalog/media-catalog.toml
```

Arguments replace that default and are forwarded to the server, so a manual
invocation does not need the mounted configuration file:

```sh
docker run --rm -p 127.0.0.1:8080:8080 \
  --user "$(id -u):$(id -g)" \
  --read-only --cap-drop ALL --security-opt no-new-privileges \
  --tmpfs /tmp:rw,noexec,nosuid,size=16m \
  -v "$PWD/media:/media:ro" \
  -v "$PWD/data:/data:ro" \
  media-catalog-server:local \
  --media-root /media --data-root /data --bind 0.0.0.0:8080
```

The bundled `config/media-catalog.toml` uses container paths:

```toml
media_root = "/media"
data_root = "/data"
bind = "0.0.0.0:8080"
max_streams = 8
```

The bind address must not stay on loopback, because container loopback is not
reachable from the host. The server's default remains `127.0.0.1:8080`.

## Compose example

```sh
mkdir -p media data
cp .env.example .env
# Edit .env if the host UID/GID is not 1000:1000.
docker compose config
docker compose up -d --build
docker compose ps
curl -i http://127.0.0.1:8080/healthz
```

`docker compose config` must list exactly one service, `server`, and resolve the
three mounts and the token environment value. `compose.yaml` publishes
`127.0.0.1:8080:8080`, so the server is not exposed beyond the host by default.

## Mount contract

| Host source | Container path | Mode | Contents |
| --- | --- | --- | --- |
| `./config/media-catalog.toml` | `/etc/media-catalog/media-catalog.toml` | read-only file | server configuration |
| `./media` | `/media` | read-only | original archive |
| `./data` | `/data` | read-only | `catalog/` JSON and `thumbs/` |
| state root (for example `./state`) | not mounted | absent | importer-only `state.sqlite` and `build.lock` |

The server needs only `media_root` and `data_root`. It must never open or mount
the state root; that directory and `build.lock` belong to the importer.

Host directories must exist before starting the container. Docker creates empty
directories for missing bind mount sources on some platforms, which then hides
the real data.

## Permissions

- The image defaults to `10001:10001`. Compose overrides the UID/GID with
  `PUID`/`PGID` from `.env`, defaulting to `1000:1000` so host files are
  readable on Linux.
- On Linux, make sure the importer writes catalog, thumbnails, and media files
  readable by that UID/GID (matching ownership or a shared group).
- The server never needs write permission on `/media`, `/data`, or the config.
- Docker Desktop maps container users differently from Linux and may ignore host
  ownership. If the server reports the media root as unavailable, adjust the
  mount permissions or use a Linux host for strict ownership checks.
- Do not run the server container as root to work around permissions.

## Readiness and the health check

`/healthz` returns `200` only when both the catalog and the media root are
usable. Before the first import it returns `503` with `catalog unavailable`;
if only the media mount is missing it reports `media unavailable`. The bootstrap
page at `/` remains available, so an empty deployment is not a crash.

The health check script is token-aware. It sends `Authorization: Bearer` only
when `MEDIA_CATALOG_TOKEN` is non-empty, and the token is passed through the
environment rather than the command line or image.

```sh
docker inspect --format '{{.State.Health.Status}}' "$(docker compose ps -q server)"
docker exec "$(docker compose ps -q server)" /usr/local/bin/healthcheck.sh
```

A `503` makes the container unhealthy. Docker does not restart a container
because it is unhealthy; the `restart: unless-stopped` policy only reacts to
process exits. Set `MEDIA_CATALOG_HEALTH_URL` when the server binds a different
port inside the container.

## External import prerequisite

The container does not import anything. Produce the catalog with the host
binary (or a separate importer deployment), using the same host directories that
are mounted into the container:

```sh
media-catalog \
  --media-root "$PWD/media" \
  --data-root "$PWD/data" \
  --state-root "$PWD/state"
```

The importer writes shards and thumbnails first and `data/catalog/index.json`
last. A running server detects the completed catalog and swaps in the new
snapshot without a restart. Keep `state/` on local disk for SQLite WAL; it stays
with the importer and is not part of the container stack.

The server can start before the first catalog exists. It serves the embedded
bootstrap page and begins rendering pages when the importer publishes a complete
`catalog/index.json`.

## Token and reverse proxy

Access control is optional and server-only:

- `MEDIA_CATALOG_TOKEN` empty or unset: every route is served without auth.
- Non-empty: every route, including `/healthz`, requires
  `Authorization: Bearer <token>`; unauthenticated requests get `401`.

The token is never baked into the image or `config/media-catalog.toml`. Keep it
in `.env` or your deployment's secret mechanism.

Browsers cannot attach a bearer header to ordinary navigation. For remote
access, keep the token empty on a trusted network or put a TLS-terminating
reverse proxy in front of the server that injects `Authorization` and forwards
`Range` and `If-Range` without buffering `/media/`. This is not a login system.
If a proxy joins the Compose network, remove the published port so only the
proxy can reach the server.

## Rollback

The container changes only where the server runs, not the catalog format:

```sh
docker compose down
./target/release/media-catalog-server \
  --data-root ./data --media-root ./media --bind 127.0.0.1:8080
```

Leave `media/`, `data/`, and `state/` untouched. No data migration is required.
