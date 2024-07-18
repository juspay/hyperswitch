# List available recipes
list:
    @just --list --justfile {{ source_file() }}

fmt_flags := '--all'

# Run formatter
fmt *FLAGS:
    cargo +nightly fmt {{ fmt_flags }} {{ FLAGS }}

check_flags := '--all-targets'

# Check compilation of Rust code
check *FLAGS:
    cargo check {{ check_flags }} {{ FLAGS }}

alias c := check

# Check compilation of Rust code and catch common mistakes
# We cannot run --all-features because v1 and v2 are mutually exclusive features
# Create a list of features by exlcuding certain features 
clippy *FLAGS:
    #! /usr/bin/env bash
    set -euo pipefail

    FEATURES="$(cargo metadata --all-features --format-version 1 | \
        jq -r '
            [ ( .workspace_members | sort ) as $package_ids # Store workspace crate package IDs in `package_ids` array
            | .packages[] | select( IN(.id; $package_ids[]) ) | .features | keys[] ] | unique # Select all unique features from all workspace crates
            | del( .[] | select( any( . ; . == ("v2", "merchant_account_v2", "payment_v2") ) ) ) # Exclude some features from features list
            | join(",") # Construct a comma-separated string of features for passing to `cargo`
    ')"
    cargo clippy {{ check_flags }} --features "${FEATURES}"  {{ FLAGS }}

alias cl := clippy

# Build binaries
build *FLAGS:
    cargo build {{ FLAGS }}

alias b := build

# Build release (optimized) binaries
build-release *FLAGS:
    cargo build --release --features release {{ FLAGS }}

alias br := build-release

# Run server
run *FLAGS:
    cargo run {{ FLAGS }}

alias r := run

doc_flags := '--all-features --all-targets --exclude-features "v2 merchant_account_v2 payment_v2"'

# Generate documentation
doc *FLAGS:
    cargo doc {{ doc_flags }} {{ FLAGS }}

alias d := doc

# Build the `euclid_wasm` crate
euclid-wasm features='dummy_connector':
    wasm-pack build \
        --target web \
        --out-dir {{ source_directory() }}/wasm \
        --out-name euclid \
        {{ source_directory() }}/crates/euclid_wasm \
        --features '{{ features }}'

# Run pre-commit checks
precommit: fmt clippy

hack_flags := '--workspace --each-feature --all-targets --exclude-features "v2 merchant_account_v2 payment_v2"'

# Check compilation of each cargo feature
hack:
    cargo hack check {{ hack_flags }}

# Check compilation of v2 feature on base dependencies
v2_intermediate_features := "merchant_account_v2,payment_v2"
hack_v2:
    cargo hack check  --feature-powerset --ignore-unknown-features --at-least-one-of "v2 " --include-features "v2" --include-features {{ v2_intermediate_features }} --package "hyperswitch_domain_models" --package "diesel_models" --package "api_models"
    cargo hack check --features "v2,payment_v2" -p storage_impl

# Use the env variables if present, or fallback to default values

db_user := env_var_or_default('DB_USER', 'db_user')
db_password := env_var_or_default('DB_PASSWORD', 'db_pass')
db_host := env_var_or_default('DB_HOST', 'localhost')
db_port := env_var_or_default('DB_PORT', '5432')
db_name := env_var_or_default('DB_NAME', 'hyperswitch_db')
default_db_url := 'postgresql://' + db_user + ':' + db_password + '@' + db_host + ':' + db_port / db_name
database_url := env_var_or_default('DATABASE_URL', default_db_url)
default_migration_params := ''
v2_migration_dir := source_directory() / 'v2_migrations'
v1_migration_dir := source_directory() / 'migrations'
resultant_dir := source_directory() / 'final-migrations'

# Copy v1 and v2 migrations to a single directory
[private]
copy_migrations:
    @mkdir -p {{ resultant_dir }}
    @cp -r {{ v1_migration_dir }}/. {{ v2_migration_dir }}/. {{ resultant_dir }}/
    echo "Created {{ resultant_dir }}"

# Delete the newly created directory
[private]
delete_dir_if_exists dir=resultant_dir:
    @rm -rf {{ dir }}

v1_config_file_dir := source_directory() / 'diesel.toml'
default_operation := 'run'

[private]
run_migration operation=default_operation migration_dir=v1_migration_dir config_file_dir=v1_config_file_dir url=database_url *other_params=default_migration_params:
    diesel migration \
        --database-url '{{ url }}' \
        {{ operation }} \
        --migration-dir '{{ migration_dir }}' \
        --config-file '{{ config_file_dir }}' \
        {{ other_params }}

# Run database migrations for v1
migrate operation=default_operation *args='': (run_migration operation v1_migration_dir v1_config_file_dir database_url args)

v2_config_file_dir := source_directory() / 'diesel_v2.toml'

# Run database migrations for v2
migrate_v2 operation=default_operation *args='':
    #! /usr/bin/env bash
    set -euo pipefail

    EXIT_CODE=0
    just copy_migrations 
    just run_migration {{ operation }} {{ resultant_dir }} {{ v2_config_file_dir }} {{ database_url }} {{ args }} || EXIT_CODE=$?
    just delete_dir_if_exists
    exit $EXIT_CODE

# Drop database if exists and then create a new 'hyperswitch_db' Database
resurrect:
    psql -U postgres -c 'DROP DATABASE IF EXISTS  hyperswitch_db';
    psql -U postgres -c 'CREATE DATABASE hyperswitch_db';

ci:
    #! /usr/bin/env bash
    set -euo pipefail

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

    # Print and execute the commands
    for command in "${commands[@]}"; do
        echo $command
        eval $command
    done
