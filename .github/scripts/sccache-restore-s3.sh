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

human() { numfmt --to=iec --suffix=B -- "$1" 2>/dev/null || printf '%s bytes' "$1"; }

mkdir -p "$SCCACHE_DIR"

if [ -z "${CACHE_S3_BUCKET:-}" ]; then
  echo "::warning::S3 cache bucket not configured on this runner; sccache starts cold"
  exit 0
fi

# Streamed, not written to disk first — avoids doubling disk usage.
#
# NOTE: this runs from `if` and `||` contexts below, which suspend `set -e` for
# the whole function body — so every failure has to `return` explicitly rather
# than relying on the shell to abort.
restore() {
  local key="$1"
  local s3_key="${CACHE_S3_KEY_PREFIX}${key}"
  local compressed_bytes start_ns elapsed_ns uncompressed_bytes

  echo "Restoring sccache cache, key: ${key}"

  start_ns="$(date +%s%N)"

  # aws's own `--progress` is unusable here: it writes carriage-return updates
  # that a non-TTY CI log renders as thousands of lines. Size, duration and
  # throughput answer the question it would have — is a slow step slow, or just
  # large?
  aws s3 cp \
    "s3://${CACHE_S3_BUCKET}/${s3_key}" \
    - \
    --region "${CACHE_S3_REGION}" --no-progress --only-show-errors \
    | tar xzf - -C "$SCCACHE_DIR" || return 1

  elapsed_ns=$(( $(date +%s%N) - start_ns ))
  uncompressed_bytes="$(du -sb "${SCCACHE_DIR}" | cut -f1)"

  # Deliberately after the download, and non-fatal: `cp` stays the single
  # arbiter of hit-vs-miss. Making HEAD the existence check would mean an
  # endpoint that answers it differently turns every restore into a silent cold
  # start — the transfer size is not worth that failure mode.
  compressed_bytes="$(
    aws s3api head-object \
      --bucket "${CACHE_S3_BUCKET}" --key "${s3_key}" \
      --region "${CACHE_S3_REGION}" \
      --query 'ContentLength' --output text 2>/dev/null
  )" || compressed_bytes=''
  if [[ "${compressed_bytes}" == 'None' ]]; then
    compressed_bytes=''
  fi

  if [ -n "${compressed_bytes}" ]; then
    echo "  downloaded:  $(human "${compressed_bytes}")"
  fi
  echo "  on disk:     $(human "${uncompressed_bytes}")"
  # An unset `c` reads as 0 in awk, which the guards below already skip.
  awk -v u="${uncompressed_bytes}" -v c="${compressed_bytes}" -v ns="${elapsed_ns}" 'BEGIN {
    s = ns / 1e9
    if (c > 0 && u > 0) {
      printf "  compressed:  %.2fx (%.1f%% smaller)\n", u / c, (1 - c / u) * 100
    }
    printf "  download:    %.1fs", s
    if (s > 0 && c > 0) { printf " at %.1f MiB/s", c / 1048576 / s }
    printf "\n"
  }'

  # Explicit: the caller reads a non-zero return as a cache miss, so the
  # function must not leak the status of whatever reporting ran last.
  return 0
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
