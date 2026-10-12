#!/usr/bin/env bash
set -euo pipefail
cd -- "$(dirname -- "${BASH_SOURCE[0]}")"
# Pinned k6 and SQLite dependencies (including transitives) live in go.mod/go.sum.
# CI and the build container use Go 1.24.4. No xk6 installation is required.
mkdir -p ../bin
CGO_ENABLED=1 GOTOOLCHAIN=go1.24.4 go build -mod=readonly -trimpath -o ../bin/k6-sqlite ./cmd/k6-sqlite
