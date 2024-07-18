#! /usr/bin/env bash

crates_with_features="$(cargo metadata --format-version 1 --no-deps \
  | jq \
    --compact-output \
    --monochrome-output \
    --raw-output \
    '[ ( .workspace_members | sort ) as $package_ids | .packages[] | select( IN( .id; $package_ids[] ) ) | { name: .name, features: ( .features | keys ) } ]')"

commands=()

# Process the metadata to generate the cargo check commands for crates which have v1 features
# We need to always have the v1 feature with each feature
# This is because, no
while IFS=' ' read -r crate features; do
  command="cargo check --all-targets --package \"${crate}\" --no-default-features --features \"${features}\""
  commands+=("$command")
done < <(jq --monochrome-output --raw-output \
  --argjson crates_with_features "${crates_with_features}" \
  --null-input \
  '$crates_with_features[] 
    | select( IN("v1"; .features[]))  # Select crates with `v1` feature
    | { name, features: (.features - ["v1", "v2", "default", "payment_v2", "merchant_account_v2"]) }  # Remove specific features to generate feature combinations
    | { name, features: ( .features | map([., "v1"] | join(",")) ) }  # Add `v1` to remaining features and join them by comma
    | .name as $name | .features[] | { $name, features: . }  # Expand nested features object to have package - features combinations
    | "\(.name) \(.features)"')  # Print out package name and features separated by space

echo "Compiling crates with v1 feature"
printf "%s\n" "${commands[@]}"

other_commands=()
 
while IFS=' ' read -r crate ; do
  command="cargo hack check --all-targets --each-feature --package \"${crate}\""
  other_commands+=("$command")
done < <(jq --monochrome-output --raw-output \
--argjson crates_with_features "${crates_with_features}" \
--null-input \
'$crates_with_features[] | select(IN("v1"; .features[]) | not ) # Select crates without `v1` feature
  | "\(.name)" # Print out package name and features separated by space')

echo "Compiling crates without v1 feature"
printf "%s\n" "${other_commands[@]}"

Print and execute the commands
for command in "${commands[@]}"; do
  echo $command
  eval $command
done