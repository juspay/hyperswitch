#! /usr/bin/env bash
#
# Restores ~/.cargo/{registry,git} and target/ from S3 as plain tarballs.
# Swatinem/rust-cache has no S3 backend, so this replaces it entirely for
# self-hosted-runner jobs, using this pod's SCCACHE_BUCKET/SCCACHE_REGION/
# SCCACHE_S3_KEY_PREFIX env vars (IAM-role auth, no credentials here).
#
# Keyed by rustc's exact build + a Cargo.lock hash (mirrors Swatinem/
# rust-cache's own key shape) so a toolchain bump or lockfile change can't
# restore incompatible artifacts silently. Falls back to the job's "latest"
# tarball on a miss, then to a true cold start.
#
# Usage: s3-cache-restore.sh <job-name>
#   Exports CACHE_KEY and CACHE_HIT (true/false) via $GITHUB_ENV.

set -euo pipefail

if [[ "${CI:-false}" != "true" && "${GITHUB_ACTIONS:-false}" != "true" ]]; then
  echo "This script is to be run in a GitHub Actions runner only. Exiting."
  exit 1
fi

job_name="$1"

echo "::add-mask::${SCCACHE_BUCKET}"
mkdir -p "$HOME/.cargo/registry" "$HOME/.cargo/git" "$GITHUB_WORKSPACE/target"

rustc_hash=$(rustc --version --verbose | awk -F': ' '/^commit-hash:/{print substr($2,1,12)}')
lock_hash=$(sha256sum Cargo.lock | cut -c1-16)
cache_key="${rustc_hash:-unknown}-${lock_hash}"
echo "CACHE_KEY=${cache_key}" >> "$GITHUB_ENV"

job_prefix="${SCCACHE_S3_KEY_PREFIX}rust-cache/${job_name}"
exact="${job_prefix}/by-key/${cache_key}"
latest="${job_prefix}/latest"

restore_from() {
  aws s3 cp --region "$SCCACHE_REGION" "s3://${SCCACHE_BUCKET}/$1/cargo.tar.gz" /tmp/cargo.tar.gz --only-show-errors \
    && aws s3 cp --region "$SCCACHE_REGION" "s3://${SCCACHE_BUCKET}/$1/target.tar.gz" /tmp/target.tar.gz --only-show-errors \
    && tar -xzf /tmp/cargo.tar.gz -C "$HOME/.cargo" \
    && tar -xzf /tmp/target.tar.gz -C "$GITHUB_WORKSPACE"
}

if restore_from "$exact"; then
  echo "CACHE_HIT=true" >> "$GITHUB_ENV"
  echo "::notice::S3 cache exact hit for ${cache_key}"
elif restore_from "$latest"; then
  echo "CACHE_HIT=false" >> "$GITHUB_ENV"
  echo "::notice::No exact S3 cache for ${cache_key} — restored latest instead"
else
  echo "CACHE_HIT=false" >> "$GITHUB_ENV"
  echo "::notice::No S3 cache available — starting cold"
fi
