#! /usr/bin/env bash
#
# Restores ~/.cargo/{registry,git} and target/ from S3 as plain tarballs.
# Swatinem/rust-cache has no S3 backend, so this replaces it entirely for
# self-hosted-runner jobs, using this pod's SCCACHE_BUCKET/SCCACHE_REGION/
# SCCACHE_S3_KEY_PREFIX env vars (IAM-role auth, no credentials here).
#
# Keyed by OS + arch + rustc's exact build + a Cargo.lock hash (mirrors
# Swatinem/rust-cache's own key shape, which also bakes in Linux-arm64
# alongside the toolchain). rustc's commit-hash alone is identical across
# every target platform for a given release — it does NOT distinguish
# arm64 from x64 — so RUNNER_OS/RUNNER_ARCH are included explicitly to
# stop artifacts from one architecture ever being restored into a job on
# another. A single by-key/<CACHE_KEY> path, no separate "latest" pointer:
# a branch whose OS/arch/rustc/Cargo.lock all match main's computes the
# exact same key and hits the exact same object main saved, directly. A
# branch that differs (a Cargo.lock bump, a toolchain rollover) misses
# here and does one clean build; the save script then populates this
# exact key so the *next* push to the same branch hits it directly too.
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
cache_key="${RUNNER_OS}-${RUNNER_ARCH}-${rustc_hash:-unknown}-${lock_hash}"
echo "CACHE_KEY=${cache_key}" >> "$GITHUB_ENV"

exact="${SCCACHE_S3_KEY_PREFIX}rust-cache/${job_name}/by-key/${cache_key}"

if aws s3 cp --region "$SCCACHE_REGION" "s3://${SCCACHE_BUCKET}/${exact}/cargo.tar.gz" /tmp/cargo.tar.gz --only-show-errors \
  && aws s3 cp --region "$SCCACHE_REGION" "s3://${SCCACHE_BUCKET}/${exact}/target.tar.gz" /tmp/target.tar.gz --only-show-errors \
  && tar -xzf /tmp/cargo.tar.gz -C "$HOME/.cargo" \
  && tar -xzf /tmp/target.tar.gz -C "$GITHUB_WORKSPACE"; then
  echo "CACHE_HIT=true" >> "$GITHUB_ENV"
  echo "::notice::S3 cache hit for ${cache_key}"
else
  echo "CACHE_HIT=false" >> "$GITHUB_ENV"
  echo "::notice::No S3 cache for ${cache_key} — starting cold"
fi
