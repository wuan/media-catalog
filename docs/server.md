# Runtime server

`media-catalog-server` is a small read-only HTTP service that serves the generated
`public/` tree and streams originals from a media root. It exists for deployments
where a web server cannot mount the media share directly.

## Topology

All-in-one, on a host that can read the media root:

```text
Browser
   |
   v
+--------------------------------------------------+
| media-catalog-server                             |
|  /                  -> public/                   |
|  /assets, /catalog  -> public/                   |
|  /thumbs/*          -> public/thumbs/*           |
|  /media/*           -> media_root/*              |
|  /healthz           -> readiness                 |
+--------------------------+-----------------------+
                           | read-only
                           v
                    media mount (SMB/NFS/local)
```

If the frontend host cannot mount the share, run the server as a media gateway on
a host that can, and reverse-proxy `/media/` to it.

Keep `public/` and `state.sqlite` on local disk. SQLite WAL is not supported on
network filesystems, and even rollback-journal mode is unreliable over SMB. Only
the original media needs to live on the NAS.

## Build and run

```sh
cargo build --release --bin media-catalog-server

export MEDIA_CATALOG_TOKEN='choose-a-long-random-value' # optional
./target/release/media-catalog-server \
  --catalog-root "$HOME/.media-catalog" \
  --media-root /Volumes/media/photos \
  --bind 127.0.0.1:8080
```

The catalog root must contain `public/`. The service binds to loopback by
default. Add `--bind 0.0.0.0:8080` or a Tailscale address only when you intend
to expose it.

The access token is read from `MEDIA_CATALOG_TOKEN`, never from the command line.
If the variable is unset or empty, access control is disabled.

```sh
curl -H "Authorization: Bearer $MEDIA_CATALOG_TOKEN" \
  http://127.0.0.1:8080/media/2024/05/photo.jpg
```

## Endpoints and behavior

| Route | Behavior |
| --- | --- |
| `/` | generated `index.html` |
| `/{year}/`, `/{year}/{month}/` | generated pages, no directory listings |
| `/assets/*`, `/catalog/*`, `/thumbs/*` | generated static files |
| `/media/*` | read-only original stream |
| `/healthz` | `200` when catalog and media roots are available, otherwise `503` |

- `HEAD` returns headers without a body.
- `Range: bytes=start-end`, `bytes=start-`, and `bytes=-suffix` return `206` with
  `Content-Range`; unsatisfiable ranges return `416`.
- Originals return strong `ETag`/`Last-Modified` and honor `If-None-Match` and
  `If-Modified-Since` with `304`.
- Versioned thumbnails are served with an immutable cache policy; HTML/JSON
  revalidate; originals use `private, no-cache`.
- Path traversal, encoded traversal, NUL, backslash, and symlink escape are
  rejected.
- If the media root disappears, `/media/` returns `503`, not a misleading `404`.
- The service never writes to the catalog or media roots and never lists
  directories.

## Reverse proxy or media gateway

If nginx stays in front and only `/media/` needs the gateway:

```nginx
server {
    listen 80;
    server_name media.example.lan;
    root /srv/media-catalog/public;

    location /media/ {
        proxy_pass http://127.0.0.1:8080/media/;
        proxy_http_version 1.1;
        proxy_set_header Host $host;
        proxy_set_header Range $http_range;
        proxy_set_header If-Range $http_if_range;
        proxy_buffering off;
        proxy_request_buffering off;
    }

    location /thumbs/ {
        add_header Cache-Control "public, max-age=2592000, immutable";
        try_files $uri =404;
    }

    location ~* \.html$ {
        add_header Cache-Control "no-cache";
    }

    location /catalog/ {
        add_header Cache-Control "no-cache";
    }
}
```

Alternatively, proxy `/` as well and let the Rust service serve the whole tree.

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
- Prefer a reverse proxy (nginx/Caddy) for TLS. The server itself speaks plain
  HTTP in this change.
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
# static catalog
curl -i http://127.0.0.1:8080/
curl -i http://127.0.0.1:8080/catalog/index.json

# media headers, HEAD, and range
curl -I http://127.0.0.1:8080/media/2024/05/clip.mp4
curl -i -H 'Range: bytes=1000-1999' \
  http://127.0.0.1:8080/media/2024/05/clip.mp4

# conditional request
etag=$(curl -sI http://127.0.0.1:8080/catalog/index.json \
  | awk 'tolower($1)=="etag:" {print $2}' | tr -d '\r')
curl -i -H "If-None-Match: $etag" \
  http://127.0.0.1:8080/catalog/index.json

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
