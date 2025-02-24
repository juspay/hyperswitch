#! /usr/bin/env bash
set -euo pipefail

# The below script is run on the github actions CI
# Obtain a list of workspace members
workspace_members="$(
  cargo metadata --format-version 1 --no-deps |
    jq \
      --compact-output \
      --monochrome-output \
      --raw-output \
      '(.workspace_members | sort) as $package_ids | .packages[] | select(IN(.id; $package_ids[])) | .name'
)"

# Arrays to track which packages are checked or skipped
PACKAGES_CHECKED=()
PACKAGES_SKIPPED=()

# Define packages to skip processing entirely
skip_packages=("detailed_errors" "vergen" "stripe_compatibility")

# Define packages that will be combined under one group
encryption_group=("encryption_service" "key_manager" "key_manager_mtls" "key_manager_forward_x_request_id")
# We will only run key_manager once (with an aggregated set of features) when any of these change.
# (Remove the others so they are not processed individually.)
encryption_group_set=()

# Function to check if an element exists in an array
contains() {
  local needle="$1"
  shift
  for item in "$@"; do
    [[ "$item" == "$needle" ]] && return 0
  done
  return 1
}

# If we are on a pull request, then only check for packages that are modified
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
    # If this package is in the skip list, then skip even if files changed.
    if contains "${package_name}" "${skip_packages[@]}"; then
      printf '::debug::Skipping `%s` (in skip list)\n' "${package_name}"
      PACKAGES_SKIPPED+=("${package_name}")
      continue
    fi

    # For packages in the encryption group, we “remember” that one of them changed
    if contains "${package_name}" "${encryption_group[@]}"; then
      encryption_group_set+=("${package_name}")
      # (Do not process individually)
      printf '::debug::Marking `%s` for encryption group aggregation\n' "${package_name}"
      continue
    fi

    # Obtain a pipe-separated list of transitive workspace dependencies for this package
    change_paths="$(
      cargo tree --all-features --no-dedupe --prefix none --package "${package_name}" |
        grep 'crates/' |
        sort --unique |
        awk '{ printf "crates/%s\n", $1 }' |
        paste -d '|' -s -
    )"

    # Check if any modified file affects the package or any of its dependencies.
    if grep --quiet --extended-regexp "^(${change_paths})" <<<"${files_modified}"; then
      printf '::debug::Checking `%s` since a relevant file changed (paths: %s)\n' "${package_name}" "${change_paths//|/ }"
      PACKAGES_CHECKED+=("${package_name}")
    else
      printf '::debug::Skipping `%s` since no modified paths match (%s)\n' "${package_name}" "${change_paths//|/ }"
      PACKAGES_SKIPPED+=("${package_name}")
    fi
  done <<< "${workspace_members}"

  # If any of the encryption group packages changed, always run key_manager
  if ((${#encryption_group_set[@]})); then
    if ! contains "key_manager" "${PACKAGES_CHECKED[@]}"; then
      PACKAGES_CHECKED+=("key_manager")
      printf '::debug:: Aggregating encryption-related packages into `key_manager`\n'
    fi
    # Optionally, log which encryption packages were merged:
    printf '::debug:: Encryption group changed: %s\n' "${encryption_group_set[*]}"
  fi

  printf '::notice::Packages checked: %s; Packages skipped: %s\n' "${PACKAGES_CHECKED[*]}" "${PACKAGES_SKIPPED[*]}"

  packages_checked="$(jq --compact-output --null-input '$ARGS.positional' --args -- "${PACKAGES_CHECKED[@]}")"

  crates_with_features="$(cargo metadata --format-version 1 --no-deps |
    jq \
      --compact-output \
      --monochrome-output \
      --raw-output \
      --argjson packages_checked "${packages_checked}" \
      '[ ( .workspace_members | sort ) as $package_ids | .packages[] | select( IN( .name; $packages_checked[] ) ) | { name: .name, features: ( .features | keys ) } ]')"
else
  # Run for all workspace packages when not on a PR.
  crates_with_features="$(cargo metadata --format-version 1 --no-deps | jq --compact-output --monochrome-output --raw-output \
    '[ ( .workspace_members | sort ) as $package_ids | .packages[]
         | { name: .name, features: ( .features | keys ) }
     ]')"
fi

# Build commands array. We now separate commands into two groups:
# (a) For crates that support v1 and have feature combinations
# (b) For crates that do not have a v1 feature (which use cargo hack)

all_commands=()

# Process the metadata to generate the cargo check commands for crates which have v1 features
# We need to always have the v1 feature with each feature
# This is because, no crate should be run without any features
crates_with_v1_feature="$(
  jq --monochrome-output --raw-output \
    --argjson crates_with_features "${crates_with_features}" \
    --null-input \
    '$crates_with_features[]
    | select( IN("v1"; .features[]))  # Select crates with `v1` feature
    | { name, features: ( .features | del( .[] | select( test("(([a-z_]+)_)?v2|v1|default") ) ) ) }  # Remove specific features to generate feature combinations
    | { name, features: ( .features | map([., "v1"] | join(",")) ) }  # Add `v1` to remaining features and join them by comma
    | .name as $name | .features[] | { $name, features: . }  # Expand nested features object to have package - features combinations
    | "\(.name) \(.features)" # Print out package name and features separated by space'
)"

while IFS=' ' read -r crate features && [[ -n "${crate}" && -n "${features}" ]]; do
  # If somehow a skipped or aggregated package sneaked in, skip it.
  if contains "${crate}" "${skip_packages[@]}"; then
    continue
  fi
  command="cargo check --all-targets --package \"${crate}\" --no-default-features --features \"${features}\""
  all_commands+=("$command")
done <<<"${crates_with_v1_feature}"

# For crates which do not have v1 feature, we can run the usual cargo hack command
crates_without_v1_feature="$(
  jq --monochrome-output --raw-output \
    --argjson crates_with_features "${crates_with_features}" \
    --null-input \
    '$crates_with_features[] | select(IN("v1"; .features[]) | not ) # Select crates without `v1` feature
    | "\(.name)" # Print out package name'
)"

while IFS= read -r crate && [[ -n "${crate}" ]]; do
  # Again, avoid skipped or encryption-group packages.
  if contains "${crate}" "${skip_packages[@]}"; then
    continue
  fi
  if contains "${crate}" "${encryption_group[@]}" && [[ "${crate}" != "key_manager" ]]; then
    # only run key_manager for encryption group so skip individual encryption group crates other than key_manager
    continue
  fi

  command="cargo hack check --all-targets --each-feature --package \"${crate}\""
  all_commands+=("$command")
done <<<"${crates_without_v1_feature}"

if ((${#all_commands[@]} == 0)); then
  echo "There are no commands to be executed"
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
