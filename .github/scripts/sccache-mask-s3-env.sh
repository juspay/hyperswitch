#! /usr/bin/env bash

set -euo pipefail

if [[ "${CI:-false}" != "true" && "${GITHUB_ACTIONS:-false}" != "true" ]]; then
  echo "This script is to be run in a GitHub Actions runner only. Exiting."
  exit 1
fi

# Not secret, but no reason to publish the bucket layout in public PR logs
# (e.g. if a failed aws s3 cp ever echoes the URI back in an error).
[ -n "${CACHE_S3_BUCKET:-}" ] && echo "::add-mask::${CACHE_S3_BUCKET}"
[ -n "${CACHE_S3_REGION:-}" ] && echo "::add-mask::${CACHE_S3_REGION}"
[ -n "${CACHE_S3_KEY_PREFIX:-}" ] && echo "::add-mask::${CACHE_S3_KEY_PREFIX}"
