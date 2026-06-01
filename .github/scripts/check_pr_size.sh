#!/usr/bin/env bash

set -euo pipefail

POLICY_FILE="${PR_SIZE_POLICY_CONFIG:-.github/pr-size-policy.yml}"
REPOSITORY="${GITHUB_REPOSITORY:?GITHUB_REPOSITORY is required}"
PR_NUMBER="${PR_NUMBER:?PR_NUMBER is required}"
PR_AUTHOR="${PR_AUTHOR:?PR_AUTHOR is required}"

for command in gh jq ruby; do
  if ! command -v "${command}" >/dev/null 2>&1; then
    echo "::error::Required command '${command}' is not installed"
    exit 1
  fi
done

if [[ ! -f "${POLICY_FILE}" ]]; then
  echo "::error::PR size policy file not found: ${POLICY_FILE}"
  exit 1
fi

function load_policy() {
  policy_json="$(ruby -ryaml -rjson -e 'puts JSON.generate(YAML.load_file(ARGV.fetch(0)) || {})' "${POLICY_FILE}")"

  enabled="$(jq -r '.enabled // true' <<< "${policy_json}")"
  metric="$(jq -r '.metric // "changed_lines"' <<< "${policy_json}")"
  threshold="$(jq -r '.threshold // empty' <<< "${policy_json}")"
  bypass_label="$(jq -r '.bypass.label // empty' <<< "${policy_json}")"
  approving_team="$(jq -r '.bypass.approving_team // empty' <<< "${policy_json}")"
  target_authors="$(jq -r '.target_authors[]? | ascii_downcase' <<< "${policy_json}")"
}

function validate_policy() {
  if [[ "${enabled}" != "true" ]]; then
    echo "PR size policy is disabled"
    exit 0
  fi

  if [[ -z "${threshold}" || ! "${threshold}" =~ ^[0-9]+$ ]]; then
    echo "::error::PR size policy threshold must be a non-negative integer"
    exit 1
  fi

  case "${metric}" in
    changed_lines | additions | changed_files) ;;
    *)
      echo "::error::Unsupported PR size metric '${metric}'. Supported: changed_lines, additions, changed_files"
      exit 1
      ;;
  esac
}

function author_is_targeted() {
  local pr_author_lower
  pr_author_lower="$(tr '[:upper:]' '[:lower:]' <<< "${PR_AUTHOR}")"

  grep --fixed-strings --line-regexp --quiet "${pr_author_lower}" <<< "${target_authors}"
}

function measure_pr_size() {
  additions=0
  deletions=0
  changed_files=0

  while IFS=$'\t' read -r file_additions file_deletions; do
    [[ -z "${file_additions}" ]] && continue
    additions=$((additions + file_additions))
    deletions=$((deletions + file_deletions))
    changed_files=$((changed_files + 1))
  done < <(
    gh api \
      --header "Accept: application/vnd.github+json" \
      --header "X-GitHub-Api-Version: 2022-11-28" \
      --paginate \
      "/repos/${REPOSITORY}/pulls/${PR_NUMBER}/files" \
      --jq '.[] | [.additions, .deletions] | @tsv'
  )

  changed_lines=$((additions + deletions))
  case "${metric}" in
    changed_lines) size="${changed_lines}" ;;
    additions) size="${additions}" ;;
    changed_files) size="${changed_files}" ;;
  esac
}

function has_bypass_label() {
  [[ -n "${bypass_label}" ]] || return 1

  gh api \
    --header "Accept: application/vnd.github+json" \
    --header "X-GitHub-Api-Version: 2022-11-28" \
    "/repos/${REPOSITORY}/issues/${PR_NUMBER}" \
    --jq '.labels[].name' | grep --fixed-strings --line-regexp --quiet "${bypass_label}"
}

function is_user_team_member() {
  local username="${1}"
  local team_slug="${2}"
  local org_name="${REPOSITORY%%/*}"

  gh api \
    --header "Accept: application/vnd.github+json" \
    --header "X-GitHub-Api-Version: 2022-11-28" \
    "/orgs/${org_name}/teams/${team_slug}/memberships/${username}" \
    >/dev/null 2>&1
}

function has_current_admin_approval() {
  [[ -n "${approving_team}" ]] || return 1

  local head_sha
  head_sha="$(gh api \
    --header "Accept: application/vnd.github+json" \
    --header "X-GitHub-Api-Version: 2022-11-28" \
    "/repos/${REPOSITORY}/pulls/${PR_NUMBER}" \
    --jq '.head.sha')"

  local approved_reviewers
  approved_reviewers="$(gh api \
    --header "Accept: application/vnd.github+json" \
    --header "X-GitHub-Api-Version: 2022-11-28" \
    --paginate \
    "/repos/${REPOSITORY}/pulls/${PR_NUMBER}/reviews" \
    --jq '.[] | select(.user.login != null and (.state == "APPROVED" or .state == "CHANGES_REQUESTED" or .state == "DISMISSED")) | [.submitted_at, .user.login, .state, .commit_id] | @tsv' \
    | sort \
    | awk -F '\t' -v head_sha="${head_sha}" '{ state[$2] = $3; commit[$2] = $4 } END { for (user in state) if (state[user] == "APPROVED" && commit[user] == head_sha) print user }')"

  while IFS= read -r reviewer; do
    [[ -z "${reviewer}" ]] && continue
    if is_user_team_member "${reviewer}" "${approving_team}"; then
      return 0
    fi
  done <<< "${approved_reviewers}"

  return 1
}

function find_bypass_reason() {
  bypass_reason=""

  if has_bypass_label; then
    bypass_reason="label '${bypass_label}'"
    return 0
  fi

  if has_current_admin_approval; then
    bypass_reason="current approval from '${approving_team}'"
    return 0
  fi

  return 1
}

function bypass_instructions() {
  local message=""

  if [[ -n "${bypass_label}" ]]; then
    message=" Add label '${bypass_label}'"
  fi
  if [[ -n "${approving_team}" ]]; then
    if [[ -n "${message}" ]]; then
      message+=" or get a current APPROVED review from '${approving_team}'"
    else
      message=" Get a current APPROVED review from '${approving_team}'"
    fi
  fi

  printf '%s' "${message}"
}

load_policy
validate_policy

if ! author_is_targeted; then
  echo "PR author '${PR_AUTHOR}' is not covered by PR size policy"
  exit 0
fi

measure_pr_size

if [[ "${size}" -le "${threshold}" ]]; then
  echo "PR size policy passed for '${PR_AUTHOR}': ${metric}=${size}, threshold=${threshold}"
  echo "Totals: additions=${additions}, deletions=${deletions}, changed_files=${changed_files}"
  exit 0
fi

if find_bypass_reason; then
  echo "PR size policy bypassed by ${bypass_reason} for '${PR_AUTHOR}': ${metric}=${size}, threshold=${threshold}"
  exit 0
fi

message="PR size policy failed for '${PR_AUTHOR}': ${metric}=${size}, threshold=${threshold}. Totals: additions=${additions}, deletions=${deletions}, changed_files=${changed_files}.$(bypass_instructions) to bypass."
echo "::error::${message}"
exit 1
