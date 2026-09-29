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

human() { numfmt --to=iec --suffix=B -- "$1" 2>/dev/null || printf '%s bytes' "$1"; }

# PR-scoped when a PR number is given, so concurrent PRs don't clobber each
# other's — or the shared merge_group/main — cache.
key="sccache-cache/${cache_name}-${RUNNER_OS}-${RUNNER_ARCH}${pr_number:+-pr${pr_number}}.tar.gz"
s3_key="${CACHE_S3_KEY_PREFIX}${key}"
echo "Saving sccache cache, key: ${key}"

uncompressed_bytes="$(du -sb "${SCCACHE_DIR}" | cut -f1)"
echo "  on disk:    $(human "${uncompressed_bytes}")"

start_ns="$(date +%s%N)"

# Streamed, not written to disk first — avoids doubling disk usage. The cost is
# that the compressed size isn't known locally, so it's read back off the object
# below rather than buffering the stream just to count it.
tar czf - -C "${SCCACHE_DIR}" . \
  | aws s3 cp - \
    "s3://${CACHE_S3_BUCKET}/${s3_key}" \
    --region "${CACHE_S3_REGION}" --no-progress --only-show-errors

elapsed_ns=$(( $(date +%s%N) - start_ns ))

# aws's own `--progress` is unusable here: it writes carriage-return updates that
# a non-TTY CI log renders as thousands of lines. Size, duration and throughput
# answer the question it would have — is a slow step slow, or just large?
compressed_bytes="$(
  aws s3api head-object \
    --bucket "${CACHE_S3_BUCKET}" --key "${s3_key}" \
    --region "${CACHE_S3_REGION}" \
    --query 'ContentLength' --output text 2>/dev/null
)" || compressed_bytes=''

if [[ -n "${compressed_bytes}" && "${compressed_bytes}" != 'None' ]]; then
  echo "  uploaded:   $(human "${compressed_bytes}")"
  awk -v u="${uncompressed_bytes}" -v c="${compressed_bytes}" -v ns="${elapsed_ns}" 'BEGIN {
    s = ns / 1e9
    if (c > 0 && u > 0) {
      printf "  compressed: %.2fx (%.1f%% smaller)\n", u / c, (1 - c / u) * 100
    }
    printf "  upload:     %.1fs", s
    if (s > 0 && c > 0) { printf " at %.1f MiB/s", c / 1048576 / s }
    printf "\n"
  }'
else
  echo "::warning::Could not read back the uploaded object size; reporting duration only"
  awk -v ns="${elapsed_ns}" 'BEGIN { printf "  upload:     %.1fs\n", ns / 1e9 }'
fi
