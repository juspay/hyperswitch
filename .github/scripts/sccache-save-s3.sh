#! /usr/bin/env bash

set -euo pipefail

if [[ "${CI:-false}" != "true" && "${GITHUB_ACTIONS:-false}" != "true" ]]; then
  echo "This script is to be run in a GitHub Actions runner only. Exiting."
  exit 1
fi

if [ -z "${1:-}" ]; then
  echo "::error::Usage: $(basename "$0") <cache-name>"
  exit 1
fi
cache_name="$1"

# Streamed, not written to disk first — avoids doubling disk usage.
tar czf - -C "$SCCACHE_DIR" . \
  | aws s3 cp - \
    "s3://${S3_CACHE_BUCKET}/${S3_CACHE_PREFIX}sccache-cache/${cache_name}-${RUNNER_OS}-${RUNNER_ARCH}.tar.gz" \
    --region "${S3_CACHE_REGION}" --no-progress --only-show-errors
