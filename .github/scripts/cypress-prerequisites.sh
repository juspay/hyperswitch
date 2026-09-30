#!/usr/bin/env bash
#
# Decides whether the heavy Cypress suites should run for the current workflow
# event and writes the decision to $GITHUB_OUTPUT as:
#
#   run-checks  true|false  run the mandatory, mock and v2 suites
#   run-heavy   true|false  run the optional, alpha and extended suites
#
# Gating is driven by two labels that a maintainer applies to the pull request:
#
#   S-test-ready  the mandatory, mock and v2 suites run on any gated event
#   S-test-full   the optional, alpha and extended suites run, and only on the
#                 event that actually adds the label. Toggling S-test-ready
#                 therefore cannot be used to re-trigger the heaviest suites.
#
# The merge queue always runs everything. Scheduled and manually dispatched
# runs are opted into by hand and also run everything. Pull requests from forks
# are never gated, because the checkout is untrusted and secrets are not
# available for them, their jobs self skip through the RUN_TESTS variable.
#
# Usage:
#   cypress-prerequisites.sh --event <name> [--label <name>] --pr <number>
#                            --repo <owner/name> [--head-sha <sha>]
#                            [--max-runs <n>] [--window-minutes <n>]
#
# Requires GH_TOKEN in the environment.

set -euo pipefail

EVENT=""
LABEL=""
PR_NUMBER=""
REPO=""
HEAD_SHA=""
HEAD_REPO=""
BASE_REPO=""

# The heaviest suites, the ones that get their own label.
HEAVY_SUITE_LABEL="S-test-full"
# The suite everyone opts into.
GATE_LABEL="S-test-ready"

# Budget for repeated label toggles on the same commit. Both are overridable
# through repository variables so the budget can be tuned without editing this
# file.
MAX_RUNS="${MAX_RUNS:-3}"
WINDOW_MINUTES="${WINDOW_MINUTES:-60}"

# Number of completed runs of this workflow on the PR head commit inside the
# cooldown window. Only completed runs are counted so that the run asking the
# question never counts itself. If the history cannot be read we fail open,
# because a flaky API call should not be the reason a PR cannot be validated.
completed_runs_in_window() {
  if [ -z "${HEAD_SHA}" ]; then
    echo 0
    return
  fi

  local since count
  since="$(date -u -d "${WINDOW_MINUTES} minutes ago" +%Y-%m-%dT%H:%M:%SZ)"
  count="$(
    gh run list \
      --repo "${REPO}" \
      --workflow "cypress-tests-runner.yml" \
      --commit "${HEAD_SHA}" \
      --status completed \
      --created ">=${since}" \
      --limit 100 \
      --json databaseId \
      --jq 'length' 2>/dev/null || echo 0
  )"

  echo "${count:-0}"
}

# Writes all three outputs in one go, so a partially evaluated decision can
# never leave $GITHUB_OUTPUT missing a key.
decide() {
  {
    echo "run-checks=$1"
    echo "run-heavy=$2"
    echo "reason=$3"
  } >>"${GITHUB_OUTPUT}"
  echo "run-checks=$1 run-heavy=$2, $3"
}

# Reads the labels of the pull request once. Returns non zero if the lookup
# itself failed, which the caller treats as unknown rather than as absent.
pr_labels() {
  local output
  if ! output="$(gh pr view "${PR_NUMBER}" --repo "${REPO}" --json labels --jq '.labels[].name' 2>&1)"; then
    echo "::warning::Could not read the labels of pull request #${PR_NUMBER}: ${output}" >&2
    return 1
  fi

  printf '%s' "${output}"
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --event) EVENT="$2"; shift 2 ;;
    --label) LABEL="$2"; shift 2 ;;
    --pr) PR_NUMBER="$2"; shift 2 ;;
    --repo) REPO="$2"; shift 2 ;;
    --head-sha) HEAD_SHA="$2"; shift 2 ;;
    --head-repo) HEAD_REPO="$2"; shift 2 ;;
    --base-repo) BASE_REPO="$2"; shift 2 ;;
    --max-runs) MAX_RUNS="$2"; shift 2 ;;
    --window-minutes) WINDOW_MINUTES="$2"; shift 2 ;;
    *) echo "::error::Unknown argument $1"; exit 2 ;;
  esac
done

if [ -z "${EVENT}" ]; then
  echo "::error::Missing required argument --event"
  exit 2
fi

if [ -z "${GH_TOKEN:-}" ]; then
  echo "::error::GH_TOKEN is required to read labels and run history"
  exit 2
fi

# The merge queue is the gate of last resort, a PR is fully validated there
# regardless of labels.
if [ "${EVENT}" = "merge_group" ]; then
  decide true true "merge queue always runs the full suite"
  exit 0
fi

# Scheduled and manually dispatched runs are asked for by hand, so they get the
# full suite without needing a label.
if [ "${EVENT}" = "workflow_dispatch" ] || [ "${EVENT}" = "schedule" ]; then
  decide true true "${EVENT} run, the full suite was requested"
  exit 0
fi

# Anything else, a push for example, has no PR to read labels from.
if [ "${EVENT}" != "pull_request" ]; then
  decide false false "${EVENT} is not a gated event"
  exit 0
fi

# Untrusted checkout, no secrets available. RUN_TESTS already skips the jobs
# for forks, so gating on a label here would buy nothing. The heaviest suites
# stay off as well, because a fork run has no creds to run them with.
if [ -n "${HEAD_REPO}" ] && [ -n "${BASE_REPO}" ] && [ "${HEAD_REPO}" != "${BASE_REPO}" ]; then
  decide true false "PR comes from a fork, labels are not used for gating"
  exit 0
fi

if [ -z "${PR_NUMBER}" ] || [ -z "${REPO}" ]; then
  echo "::error::--pr and --repo are required for pull_request events"
  exit 2
fi

if ! LABELS="$(pr_labels)"; then
  # Unknown is not the same as absent. Running the suite is the safe answer,
  # because the alternative is silently skipping the validation of a pull
  # request that was perfectly ready for it.
  decide true true "labels could not be read, running the suite to be safe"
  exit 0
fi

if ! echo "${LABELS}" | grep --quiet --fixed-strings "${GATE_LABEL}"; then
  decide false false "PR is not labelled ${GATE_LABEL}, add the label to run the suite"
  exit 0
fi

# Cooldown, so that repeatedly adding and removing the label cannot be used to
# farm CI minutes on a single commit.
COMPLETED="$(completed_runs_in_window)"
echo "${COMPLETED} of ${MAX_RUNS} runs already used on this commit in the last ${WINDOW_MINUTES} minutes."

if [ "${COMPLETED}" -ge "${MAX_RUNS}" ]; then
  decide false false "rate limited, ${MAX_RUNS} runs already used on this commit"
  exit 0
fi

# The heaviest suites are reserved for the event that adds their label. Every
# other gated event, including a plain push to a labelled PR, runs the medium
# suite only.
if [ "${LABEL}" = "${HEAVY_SUITE_LABEL}" ]; then
  decide true true "${HEAVY_SUITE_LABEL} was just added, running the optional, alpha and extended suites too"
else
  decide true false "PR is labelled ${GATE_LABEL}"
fi
