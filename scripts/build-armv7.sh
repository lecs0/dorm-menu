#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
# Requires Rust armv7-unknown-linux-musleabihf std, Zig 0.13.0 and cargo-zigbuild 0.24.1.
cargo zigbuild --target armv7-unknown-linux-musleabihf --release -p dorm-api --features nas-legacy-clock --locked
file target/armv7-unknown-linux-musleabihf/release/dorm-api
