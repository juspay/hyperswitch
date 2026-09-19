#! /usr/bin/env bash

set -euo pipefail

if [[ "${CI:-false}" != "true" && "${GITHUB_ACTIONS:-false}" != "true" ]]; then
  echo "This script is to be run in a GitHub Actions runner only. Exiting."
  exit 1
fi

# Bucket/region/prefix come from this pod's own environment (managed outside
# this repo). Captured under new names here, then blanked, so sccache itself
# never activates its native S3 backend.
echo "S3_CACHE_BUCKET=${SCCACHE_BUCKET:-}" >> "$GITHUB_ENV"
echo "S3_CACHE_REGION=${SCCACHE_REGION:-}" >> "$GITHUB_ENV"
echo "S3_CACHE_PREFIX=${SCCACHE_S3_KEY_PREFIX:-}" >> "$GITHUB_ENV"
echo "SCCACHE_BUCKET=" >> "$GITHUB_ENV"
echo "SCCACHE_REGION=" >> "$GITHUB_ENV"
echo "SCCACHE_S3_KEY_PREFIX=" >> "$GITHUB_ENV"
