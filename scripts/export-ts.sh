#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
output=$1
mkdir -p "$output/src/handlers" "$output/src/types"
cargo clean --locked --manifest-path api/Cargo.toml -p dataroom-api
TS_SERVER_FN_PACKAGE_DIR="$output" cargo check --locked --manifest-path api/Cargo.toml --features ts-bridge
TS_SERVER_FN_PACKAGE_DIR="$output" TS_RS_EXPORT_DIR="$output/src" cargo test --locked --manifest-path api/Cargo.toml --features ts-bridge --lib export_bindings_
