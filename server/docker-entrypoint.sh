#!/bin/sh
set -e

# Fix ownership on mounted volumes (Fly.io mounts as root)
if [ -d /data ]; then
    chown -R nanolambda:nanolambda /data 2>/dev/null || true
fi

# Ensure sandbox temp directory exists
mkdir -p /tmp/nanolambda/sandbox
chown -R nanolambda:nanolambda /tmp/nanolambda 2>/dev/null || true

exec "$@"
