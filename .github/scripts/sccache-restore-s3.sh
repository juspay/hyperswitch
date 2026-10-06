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

mkdir -p "$SCCACHE_DIR"

if [ -z "${CACHE_S3_BUCKET:-}" ]; then
  echo "::warning::S3 cache bucket not configured on this runner; sccache starts cold"
  exit 0
fi

# Downloaded in full, then unpacked — not piped — so a download that dies
# partway can't leave a half-populated SCCACHE_DIR for the fallback key to
# unpack on top of, and each phase stays separately timed.
archive="$(dirname "${SCCACHE_DIR}")/.sccache-restore-$$.tar"
trap 'rm -f "${archive}"' EXIT

key_for() { printf 'sccache-cache/%s-%s-%s%s.tar' "${cache_name}" "${RUNNER_OS}" "${RUNNER_ARCH}" "${1:+-pr$1}"; }
human() { numfmt --to=iec --suffix=B -- "$1"; }
rate() { awk -v b="$1" -v ns="$2" 'BEGIN { s = ns / 1e9; printf "%5.1fs", s; if (s > 0) printf " %7.0f MiB/s", b / 1048576 / s }'; }

# `set -e` is suspended in the `if`/`||` contexts below, so failures here must
# return explicitly, and the trailing `return 0` keeps the exit status of a
# reporting command from being read as a cache miss.
restore() {
  local key="$1" t0 download_ns unpack_ns archive_bytes tree_bytes

  echo "Restoring sccache cache, key: ${key}"

  # `--no-progress`: aws writes carriage-return updates that a non-TTY CI log
  # renders as thousands of lines.
  t0="$(date +%s%N)"
  aws s3 cp "s3://${CACHE_S3_BUCKET}/${CACHE_S3_KEY_PREFIX}${key}" "${archive}" \
    --region "${CACHE_S3_REGION}" --no-progress --only-show-errors || return 1
  download_ns=$(( $(date +%s%N) - t0 ))
  archive_bytes="$(stat -c%s "${archive}")"

  # Uncompressed: sccache already zstd-compresses its entries, so gzip measured
  # 1.02x here while costing more wall time than the download it was shrinking.
  t0="$(date +%s%N)"
  tar xf "${archive}" -C "${SCCACHE_DIR}" || return 1
  unpack_ns=$(( $(date +%s%N) - t0 ))

  rm -f "${archive}"
  tree_bytes="$(du -sb "${SCCACHE_DIR}" | cut -f1)"

  echo "  archive $(human "${archive_bytes}") -> tree $(human "${tree_bytes}")"
  echo "  download $(rate "${archive_bytes}" "${download_ns}")"
  echo "  unpack   $(rate "${tree_bytes}" "${unpack_ns}")"
  return 0
}

# PR-scoped first (isolates concurrent PRs from each other), falling back to the
# shared merge_group/main cache — mainly so a PR's first push isn't cold.
if [ -n "${pr_number}" ]; then
  if restore "$(key_for "${pr_number}")"; then
    exit 0
  fi
  echo "No PR-scoped cache found, falling back to shared cache"
fi

restore "$(key_for)" || echo "::warning::No sccache cache found; starting cold"
