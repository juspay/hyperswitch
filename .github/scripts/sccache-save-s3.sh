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

# shellcheck source=.github/scripts/lib-transfer-log.sh
source "$(dirname "${BASH_SOURCE[0]}")/lib-transfer-log.sh"

# PR-scoped when a PR number is given, so concurrent PRs don't clobber each
# other's — or the shared merge_group/main — cache.
key="sccache-cache/${cache_name}-${RUNNER_OS}-${RUNNER_ARCH}${pr_number:+-pr${pr_number}}.tar.zst"

tmp_archive="$(mktemp "${RUNNER_TEMP:-/tmp}/sccache-cache.XXXXXX.tar.zst")"
trap 'rm -f "$tmp_archive"' EXIT

source_bytes="$(bytes_of "$SCCACHE_DIR")"

echo "Saving sccache cache, key: ${key}"
awk -v b="$source_bytes" 'BEGIN { printf "  sccache dir            %9.1f MiB\n", b / 1048576 }'
echo "  temp space available  $(df -h --output=avail "${RUNNER_TEMP:-/tmp}" 2>/dev/null | tail -1 | tr -d ' ')"

# This was previously a single `tar | zstd | aws s3 cp -` pipeline. Two
# reasons it is now staged through a file:
#
#   1. In one pipeline the stages overlap, so there is no way to attribute
#      time to compression vs. upload — which left a 4.5 s save of a 2 GiB
#      cache unexplained.
#   2. `aws s3 cp -` reads a non-seekable stream, which the AWS CLI cannot
#      split into a parallel multipart upload. The restore path already
#      downloads to a file for the mirror-image reason.
#
# Costs one archive's worth of scratch space in RUNNER_TEMP, which is why the
# free space is logged above — if this ever fails, that line explains it.

t0=$(now_ms)
tar cf - -C "$SCCACHE_DIR" . | zstd -T0 -q -c >"$tmp_archive"
t1=$(now_ms)

archive_bytes="$(bytes_of "$tmp_archive")"
stage_report "tar + zstd" "$source_bytes" "$((t1 - t0))"
ratio_report "compression" "$source_bytes" "$archive_bytes"

t2=$(now_ms)
aws s3 cp \
  "$tmp_archive" \
  "s3://${CACHE_S3_BUCKET}/${CACHE_S3_KEY_PREFIX}${key}" \
  --region "${CACHE_S3_REGION}" 2>&1 | s3_progress
t3=$(now_ms)

stage_report "upload" "$archive_bytes" "$((t3 - t2))"
stage_report "save total" "$archive_bytes" "$((t3 - t0))"
