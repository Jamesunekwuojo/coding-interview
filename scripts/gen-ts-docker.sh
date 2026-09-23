#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
mode=${1:-write}
case "$mode" in write|check) ;; *) echo "Use write or check" >&2; exit 2 ;; esac
scratch=$(mktemp -d)
trap 'rm -rf "$scratch"' EXIT HUP INT TERM
docker compose run --rm --no-deps -v "$scratch:/generated:rw" api sh scripts/export-ts.sh /generated
docker compose run --rm --no-deps -v "$scratch:/generated:ro" -v "$PWD/api-client:/app/api-client:rw" web node scripts/finish-gen-ts.mjs /generated/src /app/api-client/src "$mode"
