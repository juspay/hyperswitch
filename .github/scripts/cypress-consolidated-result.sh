#!/usr/bin/env bash
#
# Fails or passes the aggregated Cypress result for one of the suites, so that
# the suites themselves can run as a matrix without a single flaky batch
# hiding the overall outcome.
#
# The rules are:
#
#   * a run that was never started because the PR is not labelled, or because
#     it came from a fork, is a pass, so that routine commits are not blocked
#   * a run that was skipped for a base repository pull request is a failure,
#     which is what stops an unvalidated PR from entering the merge queue
#   * a suite that ran and passed is a pass, anything else is a failure
#   * on a failure the gating label is removed from the PR, so that the author
#     has to ask for the suite again after fixing the failure
#
# A skipped result is the signature of a pull request from a fork, where the
# suite was never eligible to run in the first place. Those pass.
#
# Usage:
#   cypress-consolidated-result.sh --suite <name> --result <result>
#                                  --run-checks <bool> --event <name>
#                                  [--pr <number>] [--repo <owner/name>]
#
# Requires GH_TOKEN in the environment when a label has to be removed.

set -euo pipefail

SUITE=""
RESULT=""
RUN_CHECKS=""
EVENT=""
PR_NUMBER=""
REPO=""
GATE_LABEL="S-test-ready"

fail() {
  echo "::error::$1"
  exit 1
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --suite) SUITE="$2"; shift 2 ;;
    --result) RESULT="$2"; shift 2 ;;
    --run-checks) RUN_CHECKS="$2"; shift 2 ;;
    --event) EVENT="$2"; shift 2 ;;
    --pr) PR_NUMBER="$2"; shift 2 ;;
    --repo) REPO="$2"; shift 2 ;;
    *) fail "Unknown argument $1" ;;
  esac
done

for arg in SUITE RESULT RUN_CHECKS EVENT; do
  if [ -z "${!arg}" ]; then
    fail "Missing required argument --$(echo "${arg}" | tr '[:upper:]' '[:lower:]' | tr '_' '-')"
  fi
done

echo "${SUITE} result: ${RESULT} (run-checks=${RUN_CHECKS}, event=${EVENT})"

# The suite never started. On a pull request from the base repository this
# means the PR is not labelled, and that is a failure so the PR cannot be
# merged without the suite having run. Anywhere else, a scheduled or a fork
# run for example, there is nothing to report.
if [ "${RUN_CHECKS}" != "true" ]; then
  if [ "${EVENT}" = "pull_request" ]; then
    fail "${SUITE} did not run, add the ${GATE_LABEL} label to the pull request to run it"
  fi

  echo "${SUITE} was not scheduled for this run, treating it as a pass."
  exit 0
fi

if [ "${RESULT}" = "success" ] || [ "${RESULT}" = "skipped" ]; then
  echo "All ${SUITE} tests passed."
  exit 0
fi

echo "::error::${SUITE} tests failed."

# A merge queue failure has no pull request to relabel.
if [ "${EVENT}" != "merge_group" ] && [ -n "${PR_NUMBER}" ] && [ -n "${GH_TOKEN:-}" ]; then
  echo "Removing '${GATE_LABEL}' from the pull request."
  gh pr edit "${PR_NUMBER}" --repo "${REPO}" --remove-label "${GATE_LABEL}" || true
fi

exit 1
