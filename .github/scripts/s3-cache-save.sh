#! /usr/bin/env bash
#
# Saves ~/.cargo/{registry,git} and target/ to S3 as plain tarballs, under
# the CACHE_KEY exported by s3-cache-restore.sh earlier in the same job.
#
# Usage: s3-cache-save.sh <job-name> <update-latest: true|false>
#   update-latest=true also refreshes the job's "latest" pointer — only
#   main pushes should do this; PR saves (populating a new Cargo.lock's
#   first-ever generation) must not, or one PR's lockfile would become
#   every other PR's fallback.

set -euo pipefail

if [[ "${CI:-false}" != "true" && "${GITHUB_ACTIONS:-false}" != "true" ]]; then
  echo "This script is to be run in a GitHub Actions runner only. Exiting."
  exit 1
fi

job_name="$1"
update_latest="$2"

tar -czf /tmp/cargo.tar.gz -C "$HOME/.cargo" registry git
tar -czf /tmp/target.tar.gz -C "$GITHUB_WORKSPACE" target

# No pruning of stale dependency-version artifacts in target/ yet (cargo
# doesn't do this itself, and neither do we) — logging sizes here so growth
# over time is visible before deciding whether that's worth building.
echo "::notice::S3 cache tarball sizes for ${job_name} (${CACHE_KEY}): cargo=$(du -h /tmp/cargo.tar.gz | cut -f1), target=$(du -h /tmp/target.tar.gz | cut -f1)"

job_prefix="${SCCACHE_S3_KEY_PREFIX}rust-cache/${job_name}"
dests=("${job_prefix}/by-key/${CACHE_KEY}")
if [[ "$update_latest" == "true" ]]; then
  dests+=("${job_prefix}/latest")
fi

for dest in "${dests[@]}"; do
  aws s3 cp --region "$SCCACHE_REGION" /tmp/cargo.tar.gz "s3://${SCCACHE_BUCKET}/${dest}/cargo.tar.gz" --only-show-errors
  aws s3 cp --region "$SCCACHE_REGION" /tmp/target.tar.gz "s3://${SCCACHE_BUCKET}/${dest}/target.tar.gz" --only-show-errors
done
