#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
root=$(pwd)
mode=${1:-write}
case "$mode" in write|check) ;; *) echo "Use write or check" >&2; exit 2 ;; esac
scratch=$(mktemp -d)
trap 'rm -rf "$scratch"' EXIT HUP INT TERM
output="$scratch/client"
mkdir -p "$output/src/handlers" "$output/src/types"
sh scripts/export-ts.sh "$output"
node scripts/finish-gen-ts.mjs "$output/src" "$root/api-client/src" "$mode"
