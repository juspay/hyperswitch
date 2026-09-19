#! /usr/bin/env bash

set -euo pipefail

if [[ "${CI:-false}" != "true" && "${GITHUB_ACTIONS:-false}" != "true" ]]; then
  echo "This script is to be run in a GitHub Actions runner only. Exiting."
  exit 1
fi

# Keyed by run id: build-hyperswitch uploads its compiled router binary
# under this same path for every job in this run to download.
mkdir -p target/debug
aws s3 cp \
  "s3://${SCCACHE_BUCKET}/${SCCACHE_S3_KEY_PREFIX}build-artifacts/${GITHUB_RUN_ID}/router" \
  target/debug/router \
  --region "${SCCACHE_REGION}" --no-progress --only-show-errors
