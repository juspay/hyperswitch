#! /usr/bin/env bash

set -euo pipefail

if [[ "${CI:-false}" != "true" && "${GITHUB_ACTIONS:-false}" != "true" ]]; then
  echo "This script is to be run in a GitHub Actions runner only. Exiting."
  exit 1
fi

if [ "$#" -ne 3 ]; then
  echo "::error::Usage: $(basename "$0") <base-sha> <pr-number> <same-repo>"
  exit 1
fi
base_sha="$1"
pr_number="$2"
same_repo="$3"

# Writes `scope` to GITHUB_OUTPUT: the PR number for a PR-scoped cache, or
# empty for the shared one. Passed as the optional second argument to
# sccache-restore-s3.sh and sccache-save-s3.sh.
#
# PRs changing third-party crates get their own key: the shared cache is built
# from the base branch, so it has none of the new dependency artifacts — scoped
# keys let reruns reuse that work. Compared against the base tip, not the merge
# base, since the shared cache tracks the base tip.
#
# Never scoped for forks: untrusted checkouts must not write cache objects.

scope=''

# Only fetch when the base commit is absent: `git fetch --depth=1` on an
# unshallow clone marks it shallow, and CI-pr's `test` job later pushes
# Cargo.lock — shallow pushes are rejected.
if [[ "${same_repo}" != 'true' ]]; then
  echo "Fork pull request; using the shared cache read-only"
elif ! git cat-file -e "${base_sha}^{commit}" 2>/dev/null \
  && ! git fetch --no-tags --depth=1 origin "${base_sha}" 2>/dev/null; then
  # Force-pushed base or shallow checkout; the shared cache is a valid fallback.
  echo "::warning::Could not fetch the base commit ${base_sha}; using the shared cache"
elif git diff --quiet "${base_sha}" HEAD -- Cargo.lock; then
  echo "Cargo.lock matches the base branch; using the shared cache"
else
  echo "Cargo.lock differs from the base branch; using a PR-scoped cache"
  scope="${pr_number}"
fi

echo "scope=${scope}" >> "${GITHUB_OUTPUT}"
