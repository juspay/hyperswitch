#! /usr/bin/env bash

# TEMPORARY DIAGNOSTIC — delete before merging.
#
# Answers one question: do `aws s3` calls from this runner actually go through
# gh-actions-proxy, or direct to S3?
#
# Background: the runner pod sets
#   NO_PROXY=...,<bucket>.s3.<region>.amazonaws.com
# intending to exempt S3 from the proxy. But botocore matches NO_PROXY
# hostnames by PARENT-DOMAIN SUFFIX, not exact hostname — so an entry equal to
# the full hostname is believed to be a no-op, leaving S3 proxied. Measured
# locally against aws-cli 2.13.37; this script re-checks it on the real runner
# with the real CLI version and the real NO_PROXY value.
#
# Never fails the step: no `set -e`, everything is timeout-bounded.

set -uo pipefail

if [[ "${CI:-false}" != "true" && "${GITHUB_ACTIONS:-false}" != "true" ]]; then
  echo "This script is to be run in a GitHub Actions runner only. Exiting."
  exit 1
fi

cache_name="${1:-check-v2}"
pr_number="${2:-}"

if [ -z "${CACHE_S3_BUCKET:-}" ]; then
  echo "::warning::S3 cache bucket not configured; nothing to diagnose"
  exit 0
fi

region="${CACHE_S3_REGION}"
host="${CACHE_S3_BUCKET}.s3.${region}.amazonaws.com"
dead="http://127.0.0.1:1" # nothing listens here: connecting => proxy was chosen
fixed_no_proxy=".s3.${region}.amazonaws.com"

# Pick a key that exists, so section 3 measures a real transfer.
key=""
for candidate in \
  "${CACHE_S3_KEY_PREFIX}sccache-cache/${cache_name}-${RUNNER_OS}-${RUNNER_ARCH}${pr_number:+-pr${pr_number}}.tar.zst" \
  "${CACHE_S3_KEY_PREFIX}sccache-cache/${cache_name}-${RUNNER_OS}-${RUNNER_ARCH}.tar.zst"; do
  if aws s3api head-object --bucket "$CACHE_S3_BUCKET" --key "$candidate" \
    --region "$region" >/dev/null 2>&1; then
    key="$candidate"
    break
  fi
done

# ---------------------------------------------------------------- 1. context

echo "::group::1. Environment"
echo "aws-cli:      $(aws --version 2>&1)"
echo "HTTP_PROXY:   ${HTTP_PROXY:-${http_proxy:-<unset>}}"
echo "HTTPS_PROXY:  ${HTTPS_PROXY:-${https_proxy:-<unset>}}"
echo "NO_PROXY:     ${NO_PROXY:-${no_proxy:-<unset>}}"
echo
echo "S3 endpoint:  ${host}"
echo "resolves to:  $(getent hosts "$host" 2>/dev/null | awk '{print $1}' | paste -sd' ' -)"
echo "  (a private VPC address => interface endpoint; public => gateway/NAT)"
echo "probe key:    ${key:-<none found — section 3 will be skipped>}"
echo "::endgroup::"

# ------------------------------------------------- 2. botocore proxy decision
#
# Point the proxy vars at a dead port and ask botocore to reach S3. If it
# reports a proxy connection failure it CHOSE the proxy; any S3-level response
# (403/404) means it went direct. --no-sign-request keeps this to a single S3
# call, so a proxied STS/credential lookup can't be mistaken for the S3 one.

decision() {
  local label="$1" np="$2" out
  out="$(
    AWS_MAX_ATTEMPTS=1 \
      HTTP_PROXY="$dead" HTTPS_PROXY="$dead" http_proxy="$dead" https_proxy="$dead" \
      NO_PROXY="$np" no_proxy="$np" \
      aws s3api head-object --bucket "$CACHE_S3_BUCKET" --key "probe-nonexistent" \
      --region "$region" --no-sign-request 2>&1
  )"
  case "$out" in
  *"proxy URL"* | *ProxyConnection* | *"Cannot connect to proxy"*)
    printf '  PROXIED   %s\n' "$label" ;;
  *)
    printf '  BYPASSED  %s\n' "$label" ;;
  esac
}

echo "::group::2. Which path does botocore choose for S3?"
echo "Proxy vars pointed at a dead port; 'PROXIED' = botocore tried to use it."
echo
decision "NO_PROXY as configured on this pod  <-- the claim under test" \
  "${NO_PROXY:-${no_proxy:-}}"
decision "NO_PROXY = ${fixed_no_proxy}  (proposed fix)" \
  "${NO_PROXY:-${no_proxy:-}},${fixed_no_proxy}"
decision "NO_PROXY unset                          (control: must be PROXIED)" ""
decision "NO_PROXY = '*'                          (control: must be BYPASSED)" "*"
echo
echo "Claim is CONFIRMED if line 1 says PROXIED and line 2 says BYPASSED,"
echo "with the two controls behaving as annotated."
echo "::endgroup::"

# --------------------------------------------- 3. does a direct route exist?
#
# Prerequisite for the fix: bypassing the proxy only helps if S3 is reachable
# without it. curl --noproxy '*' forces a direct attempt regardless of env.

echo "::group::3. Is S3 reachable WITHOUT the proxy?"
direct="$(curl --noproxy '*' -sS -o /dev/null -w '%{http_code} in %{time_total}s' \
  --max-time 20 "https://${host}/" 2>&1)"
echo "  direct GET https://<bucket>/ -> ${direct}"
echo
echo "  An HTTP status (403/404 is fine) => a direct route exists, the fix is safe."
echo "  A timeout/connect error         => no route; an S3 Gateway VPC Endpoint"
echo "                                     is needed BEFORE changing NO_PROXY,"
echo "                                     or S3 access breaks entirely."
echo "::endgroup::"

# ------------------------------------------------------- 4. throughput A/B
#
# Single-connection ranged GET of the real cache object, proxied vs direct.
# One request, so this measures per-connection throughput — the number that
# explains the restore/save timings.

if [ -n "$key" ]; then
  # Resolve credentials once so each variant isn't also timing an STS call.
  eval "$(aws configure export-credentials --format env 2>/dev/null)" || true

  bytes=$((16 * 1024 * 1024))

  timed_get() {
    local label="$1" np="$2" t0 t1 ms
    t0=$(date +%s%N)
    NO_PROXY="$np" no_proxy="$np" \
      aws s3api get-object --bucket "$CACHE_S3_BUCKET" --key "$key" \
      --range "bytes=0-$((bytes - 1))" --region "$region" \
      /dev/null >/dev/null 2>&1
    local rc=$?
    t1=$(date +%s%N)
    ms=$(((t1 - t0) / 1000000))
    if [ $rc -ne 0 ]; then
      printf '  %-46s FAILED (rc=%d, %d ms)\n' "$label" "$rc" "$ms"
    else
      awk -v l="$label" -v b="$bytes" -v m="$ms" \
        'BEGIN{printf "  %-46s %7.2f MB/s  (%d ms)\n", l, b/1048576.0/(m/1000.0), m}'
    fi
  }

  echo "::group::4. Single-connection throughput, 16 MiB ranged GET"
  timed_get "NO_PROXY as configured" "${NO_PROXY:-${no_proxy:-}}"
  timed_get "NO_PROXY + ${fixed_no_proxy}" "${NO_PROXY:-${no_proxy:-}},${fixed_no_proxy}"
  echo
  echo "  In-region S3 direct should be tens of MB/s per connection."
  echo "  ~0.5-1 MB/s matches the proxied rate seen on github.com and crates.io."
  echo "::endgroup::"
fi

exit 0
