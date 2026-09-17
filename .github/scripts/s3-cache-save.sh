#! /usr/bin/env bash
#
# Saves ~/.cargo/{registry,git} and target/ to S3 as plain tarballs, under
# the CACHE_KEY exported by s3-cache-restore.sh earlier in the same job.
# main and PR branches write to the exact same by-key/<CACHE_KEY> path —
# there's no separate "latest" pointer. A branch whose key didn't exist
# yet just did one clean build; this populates that key for that branch's
# own next push to hit directly.
#
# Usage: s3-cache-save.sh <job-name>

set -euo pipefail

if [[ "${CI:-false}" != "true" && "${GITHUB_ACTIONS:-false}" != "true" ]]; then
  echo "This script is to be run in a GitHub Actions runner only. Exiting."
  exit 1
fi

job_name="$1"

tar -czf /tmp/cargo.tar.gz -C "$HOME/.cargo" registry git
tar -czf /tmp/target.tar.gz -C "$GITHUB_WORKSPACE" target

# No pruning of stale dependency-version artifacts in target/ yet (cargo
# doesn't do this itself, and neither do we) — logging sizes here so growth
# over time is visible before deciding whether that's worth building.
echo "::notice::S3 cache tarball sizes for ${job_name} (${CACHE_KEY}): cargo=$(du -h /tmp/cargo.tar.gz | cut -f1), target=$(du -h /tmp/target.tar.gz | cut -f1)"

dest="${SCCACHE_S3_KEY_PREFIX}rust-cache/${job_name}/by-key/${CACHE_KEY}"
aws s3 cp --region "$SCCACHE_REGION" /tmp/cargo.tar.gz "s3://${SCCACHE_BUCKET}/${dest}/cargo.tar.gz" --only-show-errors
aws s3 cp --region "$SCCACHE_REGION" /tmp/target.tar.gz "s3://${SCCACHE_BUCKET}/${dest}/target.tar.gz" --only-show-errors
