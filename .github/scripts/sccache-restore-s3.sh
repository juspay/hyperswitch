#! /usr/bin/env bash

set -euo pipefail

if [[ "${CI:-false}" != "true" && "${GITHUB_ACTIONS:-false}" != "true" ]]; then
  echo "This script is to be run in a GitHub Actions runner only. Exiting."
  exit 1
fi

if [ -z "${1:-}" ]; then
  echo "::error::Usage: $(basename "$0") <cache-name> [pr-number]"
  exit 1
fi
cache_name="$1"
pr_number="${2:-}"

shared_key="sccache-cache/${cache_name}-${RUNNER_OS}-${RUNNER_ARCH}.tar.gz"

mkdir -p "$SCCACHE_DIR"

if [ -z "${CACHE_S3_BUCKET:-}" ]; then
  echo "::warning::S3 cache bucket not configured on this runner; sccache starts cold"
  exit 0
fi

# Streamed, not written to disk first — avoids doubling disk usage.
restore() {
  local key="$1"
  echo "Restoring sccache cache, key: ${key}"
  aws s3 cp \
    "s3://${CACHE_S3_BUCKET}/${CACHE_S3_KEY_PREFIX}${key}" \
    - \
    --region "${CACHE_S3_REGION}" --no-progress --only-show-errors \
    | tar xzf - -C "$SCCACHE_DIR"
}

# PR-scoped first (isolates concurrent PRs from each other), falling back to
# the shared merge_group/main cache — mainly so a PR's first push isn't cold.
if [ -n "$pr_number" ]; then
  pr_key="sccache-cache/${cache_name}-${RUNNER_OS}-${RUNNER_ARCH}-pr${pr_number}.tar.gz"
  if restore "$pr_key"; then
    exit 0
  fi
  echo "No PR-scoped cache found, falling back to shared cache"
fi

restore "$shared_key" || echo "::warning::No sccache cache found; starting cold"
