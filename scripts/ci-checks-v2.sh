#! /usr/bin/env bash
set -euo pipefail

crates_to_check=\
'api_models
diesel_models
hyperswitch_domain_models
storage_impl'

v2_feature_set='v2,merchant_account_v2,payment_v2,customer_v2,routing_v2,business_profile_v2'

packages_checked=()
packages_skipped=()

# List of cargo commands that will be executed
all_commands=()

# If we are running this on a pull request, then only check for packages that are modified
if [[ "${GITHUB_EVENT_NAME:-}" == 'pull_request' ]]; then
  # Obtain the pull request number and files modified in the pull request
  pull_request_number="$(jq --raw-output '.pull_request.number' "${GITHUB_EVENT_PATH}")"
  files_modified="$(
    gh api \
      --header 'Accept: application/vnd.github+json' \
      --header 'X-GitHub-Api-Version: 2022-11-28' \
      --paginate \
      "https://api.github.com/repos/${GITHUB_REPOSITORY}/pulls/${pull_request_number}/files" \
      --jq '.[].filename'
  )"

  while IFS= read -r package_name; do
    # A package must be checked if it has been modified
    if grep --quiet --extended-regexp "^crates/${package_name}" <<< "${files_modified}"; then
      if [[ "${package_name}" == "storage_impl" ]]; then
        all_commands+=("cargo hack clippy --features 'v2,payment_v2,customer_v2' -p storage_impl")
      else
        all_commands+=("cargo hack clippy --feature-powerset --depth 2 --ignore-unknown-features --at-least-one-of 'v2 ' --include-features '${v2_feature_set}' --package '${package_name}'")
      fi
      printf '::debug::Checking `%s` since it was modified %s\n' "${package_name}"
      packages_checked+=("${package_name}")
    else
      printf '::debug::Skipping `%s` since it was not modified: %s\n' "${package_name}"
      packages_skipped+=("${package_name}")
    fi
  done <<< "${crates_to_check}"
  printf '::notice::Packages checked: %s; Packages skipped: %s\n' "${packages_checked[*]}" "${packages_skipped[*]}"

else
  # If we are doing this locally or on merge queue, then check for all the V2 crates
  all_commands+=("cargo hack clippy --features 'v2,payment_v2,customer_v2' -p storage_impl")

  common_command="cargo hack clippy --feature-powerset --depth 2 --ignore-unknown-features --at-least-one-of 'v2 ' --include-features '${v2_feature_set}'"
  crates_to_include=""
  while IFS= read -r crate; do
    if [[ "${crate}" != "storage_impl" ]]; then
      crates_to_include+="--package '${crate}' "
    fi
  done <<< "${crates_to_check}"
  all_commands+=("${common_command} ${crates_to_include}")
fi

if ((${#all_commands[@]} == 0)); then
  echo "There are no commands to be be executed"
  exit 0
fi

echo "The list of commands that will be executed:"
printf "%s\n" "${all_commands[@]}"
echo

# Execute the commands
for command in "${all_commands[@]}"; do
  if [[ "${CI:-false}" == "true" && "${GITHUB_ACTIONS:-false}" == "true" ]]; then
    printf '::group::Running `%s`\n' "${command}"
  fi

  bash -c -x "${command}"

  if [[ "${CI:-false}" == "true" && "${GITHUB_ACTIONS:-false}" == "true" ]]; then
    echo '::endgroup::'
  fi
done
