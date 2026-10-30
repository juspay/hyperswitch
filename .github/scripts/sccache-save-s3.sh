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
key="$(printf 'sccache-cache/%s-%s-%s%s.tar' "${cache_name}" "${RUNNER_OS}" "${RUNNER_ARCH}" "${pr_number:+-pr${pr_number}}")"

# Written out in full, then uploaded — not piped — so the pack and the transfer
# stay separately timed.
archive="$(dirname "${SCCACHE_DIR}")/.sccache-save-$$.tar"
trap 'rm -f "${archive}"' EXIT

human() { numfmt --to=iec --suffix=B -- "$1"; }
rate() { awk -v b="$1" -v ns="$2" 'BEGIN { s = ns / 1e9; printf "%5.1fs", s; if (s > 0) printf " %7.0f MiB/s", b / 1048576 / s }'; }

echo "Saving sccache cache, key: ${key}"

# Uncompressed: sccache already zstd-compresses its entries, so gzip measured
# 1.02x here while costing more wall time than the transfer it was shrinking.
t0="$(date +%s%N)"
tar cf "${archive}" -C "${SCCACHE_DIR}" .
pack_ns=$(( $(date +%s%N) - t0 ))

tree_bytes="$(du -sb "${SCCACHE_DIR}" | cut -f1)"
archive_bytes="$(stat -c%s "${archive}")"

# `--no-progress`: aws writes carriage-return updates that a non-TTY CI log
# renders as thousands of lines.
t0="$(date +%s%N)"
aws s3 cp "${archive}" "s3://${CACHE_S3_BUCKET}/${CACHE_S3_KEY_PREFIX}${key}" \
  --region "${CACHE_S3_REGION}" --no-progress --only-show-errors
upload_ns=$(( $(date +%s%N) - t0 ))

echo "  tree $(human "${tree_bytes}") -> archive $(human "${archive_bytes}")"
echo "  pack   $(rate "${tree_bytes}" "${pack_ns}")"
echo "  upload $(rate "${archive_bytes}" "${upload_ns}")"
