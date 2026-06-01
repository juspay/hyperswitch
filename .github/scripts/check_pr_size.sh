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

policy_json="$(ruby -ryaml -rjson -e 'puts JSON.generate(YAML.load_file(ARGV.fetch(0)) || {})' "${POLICY_FILE}")"

enabled="$(jq -r '.enabled // true' <<< "${policy_json}")"
metric="$(jq -r '.metric // "changed_lines"' <<< "${policy_json}")"
threshold="$(jq -r '.threshold // empty' <<< "${policy_json}")"
bypass_label="$(jq -r '.bypass.label // empty' <<< "${policy_json}")"
approving_team="$(jq -r '.bypass.approving_team // empty' <<< "${policy_json}")"
target_authors="$(jq -r '.target_authors[]? | ascii_downcase' <<< "${policy_json}")"

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

pr_author_lower="$(tr '[:upper:]' '[:lower:]' <<< "${PR_AUTHOR}")"
if ! grep --fixed-strings --line-regexp --quiet "${pr_author_lower}" <<< "${target_authors}"; then
  echo "PR author '${PR_AUTHOR}' is not covered by PR size policy"
  exit 0
fi

has_bypass_label=false
if [[ -n "${bypass_label}" ]]; then
  if gh api \
      --header "Accept: application/vnd.github+json" \
      --header "X-GitHub-Api-Version: 2022-11-28" \
      "/repos/${REPOSITORY}/issues/${PR_NUMBER}" \
      --jq '.labels[].name' | grep --fixed-strings --line-regexp --quiet "${bypass_label}"; then
    has_bypass_label=true
  fi
fi

function is_user_team_member() {
  local username="${1}"
  local team_slug="${2}"
  local org_name="${REPOSITORY%%/*}"

  local status_code
  status_code="$(
    curl \
      --location \
      --silent \
      --output /dev/null \
      --write-out '%{http_code}' \
      --header 'Accept: application/vnd.github+json' \
      --header 'X-GitHub-Api-Version: 2022-11-28' \
      --header "Authorization: Bearer ${GH_TOKEN}" \
      "https://api.github.com/orgs/${org_name}/teams/${team_slug}/memberships/${username}"
  )"

  [[ "${status_code}" -eq 200 ]]
}

has_admin_approval=false
if [[ -n "${approving_team}" ]]; then
  approved_reviewers="$(gh api \
    --header "Accept: application/vnd.github+json" \
    --header "X-GitHub-Api-Version: 2022-11-28" \
    --paginate \
    "/repos/${REPOSITORY}/pulls/${PR_NUMBER}/reviews" \
    --jq '.[] | select(.state == "APPROVED") | .user.login' | sort -u)"

  while IFS= read -r reviewer; do
    [[ -z "${reviewer}" ]] && continue
    if is_user_team_member "${reviewer}" "${approving_team}"; then
      has_admin_approval=true
      break
    fi
  done <<< "${approved_reviewers}"
fi

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

if [[ "${size}" -le "${threshold}" ]]; then
  echo "PR size policy passed for '${PR_AUTHOR}': ${metric}=${size}, threshold=${threshold}"
  echo "Totals: additions=${additions}, deletions=${deletions}, changed_files=${changed_files}"
  exit 0
fi

if [[ "${has_bypass_label}" == "true" ]]; then
  echo "PR size policy bypassed by label '${bypass_label}' for '${PR_AUTHOR}': ${metric}=${size}, threshold=${threshold}"
  exit 0
fi

if [[ "${has_admin_approval}" == "true" ]]; then
  echo "PR size policy bypassed by approval from '${approving_team}' for '${PR_AUTHOR}': ${metric}=${size}, threshold=${threshold}"
  exit 0
fi

bypass_message=""
if [[ -n "${bypass_label}" ]]; then
  bypass_message=" Add label '${bypass_label}'"
fi
if [[ -n "${approving_team}" ]]; then
  if [[ -n "${bypass_message}" ]]; then
    bypass_message+=" or get an APPROVED review from '${approving_team}'"
  else
    bypass_message=" Get an APPROVED review from '${approving_team}'"
  fi
fi

message="PR size policy failed for '${PR_AUTHOR}': ${metric}=${size}, threshold=${threshold}. Totals: additions=${additions}, deletions=${deletions}, changed_files=${changed_files}.${bypass_message} to bypass."
echo "::error::${message}"
exit 1
