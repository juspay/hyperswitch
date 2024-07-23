#! /usr/bin/env bash
set -euo pipefail

# The below script is run on the github actions CI
# Obtain a list of workspace members
workspace_members="$(
  cargo metadata --format-version 1 --no-deps \
    | jq \
      --compact-output \
      --monochrome-output \
      --raw-output \
      '(.workspace_members | sort) as $package_ids | .packages[] | select(IN(.id; $package_ids[])) | .name'
)"

PACKAGES_CHECKED=()
PACKAGES_SKIPPED=()

# If we are running this on a pull request, then only check for packages that are modified
if [[ -n "${GITHUB_BASE_REF:-}" ]]; then
  while IFS= read -r package_name; do
    # Obtain comma-separated list of transitive workspace dependencies for each workspace member
    change_paths="$(cargo tree --all-features --no-dedupe --prefix none --package "${package_name}" \
      | grep 'crates/' \
      | sort --unique \
      | awk --field-separator ' ' '{ printf "crates/%s\n", $1 }' | paste -d ',' -s -)"

    # Store change paths in an array by splitting `change_paths` by comma
    IFS=',' read -ra change_paths <<< "${change_paths}"

    # A package must be checked if any of its transitive dependencies (or itself) has been modified
    if git diff --exit-code --quiet "origin/${GITHUB_BASE_REF}" -- "${change_paths[@]}"; then
      printf '::debug::Skipping `%s` since none of these paths were modified: %s\n' "${package_name}" "${change_paths[*]}"
      PACKAGES_SKIPPED+=("${package_name}")
    else
      printf '::debug::Checking `%s` since at least one of these paths was modified: %s\n' "${package_name}" "${change_paths[*]}"
      PACKAGES_CHECKED+=("${package_name}")
    fi
  done <<< "${workspace_members}"
  printf '::notice::Packages checked: %s; Packages skipped: %s\n' "${PACKAGES_CHECKED[*]}" "${PACKAGES_SKIPPED[*]}"

  packages_checked=$(printf '%s\n' "${PACKAGES_CHECKED[@]}" | jq -R . | jq -s .)

  crates_with_features="$(cargo metadata --format-version 1 --no-deps \
    | jq \
      --compact-output \
      --monochrome-output \
      --raw-output \
      --argjson packages_checked "${packages_checked}" \
      '[ ( .workspace_members | sort ) as $package_ids | .packages[] | select( IN( .name; $packages_checked[] ) ) | { name: .name, features: ( .features | keys ) } ]')"
else
  # If we are doing this locally or on merge queue, then check for all the crates
  crates_with_features="$(cargo metadata --format-version 1 --no-deps \
    | jq \
      --compact-output \
      --monochrome-output \
      --raw-output \
      '[ ( .workspace_members | sort ) as $package_ids | .packages[] | select( IN( .id; $package_ids[] ) ) | { name: .name, features: ( .features | keys ) } ]')"
fi

# List of cargo commands that will be executed
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
    | { name, features: (.features - ["v1", "v2", "default", "payment_v2", "merchant_account_v2"]) }  # Remove specific features to generate feature combinations
    | { name, features: ( .features | map([., "v1"] | join(",")) ) }  # Add `v1` to remaining features and join them by comma
    | .name as $name | .features[] | { $name, features: . }  # Expand nested features object to have package - features combinations
    | "\(.name) \(.features)" # Print out package name and features separated by space'
)"
while IFS=' ' read -r crate features && [[ -n "${crate}" && -n "${features}" ]]; do
  command="cargo check --all-targets --package \"${crate}\" --no-default-features --features \"${features}\""
  all_commands+=("$command")
done <<< "${crates_with_v1_feature}"

# For crates which do not have v1 feature, we can run the usual cargo hack command
crates_without_v1_feature="$(
  jq --monochrome-output --raw-output \
    --argjson crates_with_features "${crates_with_features}" \
    --null-input \
    '$crates_with_features[] | select(IN("v1"; .features[]) | not ) # Select crates without `v1` feature
    | "\(.name)" # Print out package name'
)"
while IFS= read -r crate && [[ -n "${crate}" ]]; do
  command="cargo hack check --all-targets --each-feature --package \"${crate}\""
  all_commands+=("$command")
done <<< "${crates_without_v1_feature}"

if (( ${#all_commands[@]} == 0 )); then
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
