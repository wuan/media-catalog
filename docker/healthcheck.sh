#!/bin/sh
# Report catalog/media readiness. The bearer header is sent only when a token
# is configured, so the token never appears in the container command line.
set -eu

url="${MEDIA_CATALOG_HEALTH_URL:-http://127.0.0.1:8080/healthz}"

if [ -n "${MEDIA_CATALOG_TOKEN:-}" ]; then
    exec curl --fail --silent --show-error --max-time 5 \
        --header "Authorization: Bearer ${MEDIA_CATALOG_TOKEN}" \
        "$url"
fi

exec curl --fail --silent --show-error --max-time 5 "$url"
