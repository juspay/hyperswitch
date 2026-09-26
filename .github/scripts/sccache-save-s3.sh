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

# PR-scoped when a PR number is given, so concurrent PRs don't clobber each
# other's — or the shared merge_group/main — cache.
key="sccache-cache/${cache_name}-${RUNNER_OS}-${RUNNER_ARCH}${pr_number:+-pr${pr_number}}.tar.zst"
echo "Saving sccache cache, key: ${key}"

# zstd -T0 spreads compression across all cores; gzip (tar's -z) is
# single-threaded and becomes the throughput ceiling on large caches.
# Streamed, not written to disk first — avoids doubling disk usage.
tar cf - -C "$SCCACHE_DIR" . \
  | zstd -T0 -q -c \
  | aws s3 cp - \
    "s3://${CACHE_S3_BUCKET}/${CACHE_S3_KEY_PREFIX}${key}" \
    --region "${CACHE_S3_REGION}" --no-progress --only-show-errors
