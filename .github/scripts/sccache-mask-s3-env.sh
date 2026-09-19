#! /usr/bin/env bash

set -euo pipefail

if [[ "${CI:-false}" != "true" && "${GITHUB_ACTIONS:-false}" != "true" ]]; then
  echo "This script is to be run in a GitHub Actions runner only. Exiting."
  exit 1
fi

# Not secret, but no reason to publish the bucket layout in public PR logs
# (sccache --show-stats echoes it back).
[ -n "${SCCACHE_BUCKET:-}" ] && echo "::add-mask::${SCCACHE_BUCKET}"
[ -n "${SCCACHE_REGION:-}" ] && echo "::add-mask::${SCCACHE_REGION}"
[ -n "${SCCACHE_S3_KEY_PREFIX:-}" ] && echo "::add-mask::${SCCACHE_S3_KEY_PREFIX}"
