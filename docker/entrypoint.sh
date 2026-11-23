#!/bin/sh
# Start the catalog server. Supplied arguments replace the default command.
set -eu

if [ "$#" -gt 0 ]; then
    exec media-catalog-server "$@"
fi

exec media-catalog-server --config /etc/media-catalog/media-catalog.toml
