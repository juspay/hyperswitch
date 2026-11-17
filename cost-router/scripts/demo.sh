#!/usr/bin/env bash
# End-to-end demo of the cost-router against a running stack (run scripts/up.sh first).
# Needs only node (>=18) and a built cost-router (this script installs and builds if needed).
set -eu
cd "$(dirname "$0")/.."
[ -d node_modules ] || npm install --no-audit --no-fund
[ -f dist/main.js ] || npm run build
exec node scripts/demo.mjs "$@"
