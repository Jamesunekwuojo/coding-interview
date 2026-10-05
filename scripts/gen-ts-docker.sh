#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
root=$(pwd)
mode=${1:-write}
case "$mode" in write|check) ;; *) echo "Use write or check" >&2; exit 2 ;; esac

scratch=$(mktemp -d)
trap 'rm -rf "$scratch" 2>/dev/null || true' EXIT HUP INT TERM
chmod 777 "$scratch"
chcon -t container_file_t "$scratch" 2>/dev/null || true

docker compose run --rm --no-deps -v "$scratch:/generated:rw,z" api sh scripts/export-ts.sh /generated

docker compose run --rm --no-deps \
  -v "$scratch:/generated:ro,z" \
  -v "$root/api-client:/out-client:rw,z" \
  web node scripts/finish-gen-ts.mjs /generated/src /out-client/src "$mode"
