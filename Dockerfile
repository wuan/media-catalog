# syntax=docker/dockerfile:1

# Build stage: compile only the runtime server. Askama templates and `include_str!`
# assets are embedded at compile time, so both directories are required here.
FROM rust:1.100-bookworm AS build

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY templates ./templates
COPY assets ./assets

RUN cargo build --locked --release --bin media-catalog-server

# Runtime stage: the server, its entrypoint, the health check, and curl only.
# The importer, ffmpeg, ffprobe, and any scheduler are deliberately absent.
FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install --no-install-recommends --yes ca-certificates curl \
    && rm -rf /var/lib/apt/lists/* \
    && install -d -m 0755 /etc/media-catalog /media /data

COPY --from=build /app/target/release/media-catalog-server /usr/local/bin/media-catalog-server
COPY docker/entrypoint.sh /usr/local/bin/docker-entrypoint.sh
COPY docker/healthcheck.sh /usr/local/bin/healthcheck.sh

RUN chmod 0755 \
    /usr/local/bin/media-catalog-server \
    /usr/local/bin/docker-entrypoint.sh \
    /usr/local/bin/healthcheck.sh

# Numeric non-root user. Compose may override the UID/GID to match host files.
USER 10001:10001

EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD ["/usr/local/bin/healthcheck.sh"]

ENTRYPOINT ["/usr/local/bin/docker-entrypoint.sh"]
