#! /usr/bin/env bash
set -euo pipefail

# Parallel version of ci-checks.sh
# The below script is run on the github actions CI with parallel execution

# Get number of CPU cores, defaulting to 4 if detection fails
PARALLEL_JOBS="${PARALLEL_JOBS:-$(nproc 2>/dev/null || sysctl -n hw.ncpu 2>/dev/null || echo 4)}"
MAX_PARALLEL="${MAX_PARALLEL:-$PARALLEL_JOBS}"

echo "Running CI checks with ${MAX_PARALLEL} parallel jobs"

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
    # Obtain pipe-separated list of transitive workspace dependencies for each workspace member
    change_paths="$(cargo tree --all-features --no-dedupe --prefix none --package "${package_name}" \
      | grep 'crates/' \
      | sort --unique \
      | awk --field-separator ' ' '{ printf "crates/%s\n", $1 }' | paste -d '|' -s -)"

    # A package must be checked if any of its transitive dependencies (or itself) has been modified
    if grep --quiet --extended-regexp "^(${change_paths})" <<< "${files_modified}"; then
      printf '::debug::Checking `%s` since at least one of these paths was modified: %s\n' "${package_name}" "${change_paths[*]//|/ }"
      PACKAGES_CHECKED+=("${package_name}")
    else
      printf '::debug::Skipping `%s` since none of these paths were modified: %s\n' "${package_name}" "${change_paths[*]//|/ }"
      PACKAGES_SKIPPED+=("${package_name}")
    fi
  done <<< "${workspace_members}"
  printf '::notice::Packages checked: %s; Packages skipped: %s\n' "${PACKAGES_CHECKED[*]}" "${PACKAGES_SKIPPED[*]}"

  packages_checked="$(jq --compact-output --null-input '$ARGS.positional' --args -- "${PACKAGES_CHECKED[@]}")"

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
    | { name, features: ( .features | del( .[] | select( any( . ; test("(([a-z_]+)_)?v2|v1|default") ) ) ) ) }  # Remove specific features to generate feature combinations
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

if ((${#all_commands[@]} == 0)); then
  echo "There are no commands to be executed"
  exit 0
fi

echo "The list of commands that will be executed in parallel (max ${MAX_PARALLEL} jobs):"
printf "%s\n" "${all_commands[@]}"
echo

# Function to run a single command with proper logging
run_command() {
  local job_id="$1"
  local command="$2"
  
  if [[ "${CI:-false}" == "true" && "${GITHUB_ACTIONS:-false}" == "true" ]]; then
    printf '::group::[Job %s] Running `%s`\n' "${job_id}" "${command}"
  else
    printf '[Job %s] Running: %s\n' "${job_id}" "${command}"
  fi

  # Create a temporary file for this job's output
  local temp_output
  temp_output=$(mktemp)
  
  # Run the command and capture both stdout and stderr
  if bash -c "${command}" >"${temp_output}" 2>&1; then
    local exit_code=0
  else
    local exit_code=$?
  fi
  
  # Display the output only if there's content
  if [[ -s "${temp_output}" ]]; then
    cat "${temp_output}"
  fi
  
  # Clean up
  rm -f "${temp_output}"
  
  if [[ "${CI:-false}" == "true" && "${GITHUB_ACTIONS:-false}" == "true" ]]; then
    echo '::endgroup::'
  fi
  
  if [[ ${exit_code} -ne 0 ]]; then
    printf '[Job %s] FAILED with exit code %s: %s\n' "${job_id}" "${exit_code}" "${command}" >&2
    exit ${exit_code}
  else
    printf '[Job %s] SUCCESS: %s\n' "${job_id}" "${command}"
  fi
}

# Export the function so it can be used by parallel processes
export -f run_command

# Check if we have GNU parallel available
if command -v parallel >/dev/null 2>&1; then
  echo "Using GNU parallel for execution"
  
  # Create temporary file for commands
  temp_commands=$(mktemp)
  trap "rm -f ${temp_commands}" EXIT
  
  # Write all commands to temporary file, one per line
  printf '%s\n' "${all_commands[@]}" > "${temp_commands}"
  
  # Use GNU parallel with proper job numbering
  parallel -j "${MAX_PARALLEL}" --line-buffer --tag --jobs-log /tmp/parallel.log \
    'run_command {#} "{}"' :::: "${temp_commands}"
  
elif command -v xargs >/dev/null 2>&1; then
  echo "Using xargs for parallel execution"
  
  # Use a different approach for xargs compatibility
  job_counter=1
  for command in "${all_commands[@]}"; do
    # Escape the command properly for shell execution
    escaped_command=$(printf '%q' "${command}")
    echo "${job_counter} ${escaped_command}"
    ((job_counter++))
  done | xargs -n 2 -P "${MAX_PARALLEL}" bash -c 'run_command "$1" "$2"' _
  
else
  echo "No parallel execution tool found, falling back to sequential execution"
  
  # Final fallback: sequential execution with job numbering
  job_counter=1
  for command in "${all_commands[@]}"; do
    run_command "${job_counter}" "${command}"
    ((job_counter++))
  done
fi

echo "All CI checks completed successfully!"