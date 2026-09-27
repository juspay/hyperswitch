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

shared_key="sccache-cache/${cache_name}-${RUNNER_OS}-${RUNNER_ARCH}.tar.zst"

mkdir -p "$SCCACHE_DIR"

if [ -z "${CACHE_S3_BUCKET:-}" ]; then
  echo "::warning::S3 cache bucket not configured on this runner; sccache starts cold"
  exit 0
fi

# TEMPORARY — revert with the rest of the diagnostics.
#
# The pod's NO_PROXY lists this bucket by its full hostname, which botocore
# never matches (it compares parent-domain suffixes), so S3 currently goes
# through the proxy. Appending the suffix form that *does* match should flip
# the TCP path proof below from PROXIED to DIRECT — which is what validates
# that the proof can detect both states rather than only ever printing one.
#
# Appended rather than replacing the list: dropping the IMDS and cluster-CIDR
# entries would route credential lookups through the proxy and break auth,
# failing for a reason unrelated to what is being tested. `.s3.<region>...`
# does not match `sts.<region>...`, so credentials keep using the proxy here.
#
# Must be done here, not in the workflow's `env:` block — `${{ env.NO_PROXY }}`
# reads the workflow env context, not the runner process environment, so it
# would expand empty and silently truncate the pod's list.
export NO_PROXY="${NO_PROXY:-},.s3.${CACHE_S3_REGION}.amazonaws.com"
export no_proxy="$NO_PROXY"
echo "  [temp] NO_PROXY widened with .s3.<region>.amazonaws.com — expecting VERDICT: DIRECT"

tmp_archive="$(mktemp "${RUNNER_TEMP:-/tmp}/sccache-cache.XXXXXX.tar.zst")"
trap 'rm -f "$tmp_archive"' EXIT

# Downloaded to a real (seekable) file rather than streamed straight to
# stdout: that lets the AWS CLI fetch byte ranges over several connections
# in parallel. Streaming to stdout forces one sequential GET no matter how
# big the object is, which is far slower for multi-GB caches.
#
# Download and extract are timed separately because the step total conflates
# them, and they have completely different bottlenecks (network vs. creating
# tens of thousands of small files).
restore() {
  local key="$1"
  echo "Restoring sccache cache, key: ${key}"

  local t0 t1 t2 t3 rc=0 archive_bytes extracted_bytes sampler peers_file

  # Observe the real sockets this download uses, so "does S3 go through the
  # proxy?" is answered by evidence rather than by inference. Rerun after
  # changing the runner's proxy config and diff the VERDICT line.
  peers_file="$(mktemp "${RUNNER_TEMP:-/tmp}/sccache-peers.XXXXXX")"
  sampler="$(peer_sampler_start "$peers_file")"

  t0=$(now_ms)
  aws s3 cp \
    "s3://${CACHE_S3_BUCKET}/${CACHE_S3_KEY_PREFIX}${key}" \
    "$tmp_archive" \
    --region "${CACHE_S3_REGION}" 2>&1 | s3_progress || rc=$?
  t1=$(now_ms)

  peer_verdict "$sampler" "$peers_file" \
    "${HTTPS_PROXY:-${https_proxy:-${HTTP_PROXY:-${http_proxy:-http://none}}}}"
  rm -f "$peers_file"

  if [ "$rc" -ne 0 ]; then
    return "$rc"
  fi

  archive_bytes="$(bytes_of "$tmp_archive")"
  stage_report "download" "$archive_bytes" "$((t1 - t0))"

  t2=$(now_ms)
  zstd -d -q -c "$tmp_archive" | tar xf - -C "$SCCACHE_DIR" || rc=$?
  t3=$(now_ms)

  if [ "$rc" -ne 0 ]; then
    echo "::warning::sccache archive failed to extract (rc=${rc})"
    return "$rc"
  fi

  extracted_bytes="$(bytes_of "$SCCACHE_DIR")"
  stage_report "decompress + extract" "$extracted_bytes" "$((t3 - t2))"
  ratio_report "compression" "$extracted_bytes" "$archive_bytes"
  stage_report "restore total" "$archive_bytes" "$((t3 - t0))"
  return 0
}

# PR-scoped first (isolates concurrent PRs from each other), falling back to
# the shared merge_group/main cache — mainly so a PR's first push isn't cold.
if [ -n "$pr_number" ]; then
  pr_key="sccache-cache/${cache_name}-${RUNNER_OS}-${RUNNER_ARCH}-pr${pr_number}.tar.zst"
  if restore "$pr_key"; then
    exit 0
  fi
  echo "No PR-scoped cache found, falling back to shared cache"
fi

restore "$shared_key" || echo "::warning::No sccache cache found; starting cold"
