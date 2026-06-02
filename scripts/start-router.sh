#!/usr/bin/env bash
# Launch the hyperswitch router with a larger default thread stack so actix
# worker threads (spawned via std::thread::Builder, which respects
# RUST_MIN_STACK) do not stack-overflow on deeply nested generic dispatch
# paths used by UCS-only connectors (e.g. tsys_xml /payments).
#
# 16 MiB matches the tokio runtime's thread_stack_size set in router.rs.
set -euo pipefail

export RUST_MIN_STACK="${RUST_MIN_STACK:-16777216}"

BIN="${ROUTER_BIN:-./target/debug/router}"
exec "$BIN" "$@"
