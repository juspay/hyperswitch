# List available recipes
list:
    @just --list --justfile {{ source_file() }}

fmt_flags := '--all'

# Run formatter
fmt *FLAGS:
    cargo +nightly fmt {{ fmt_flags }} {{ FLAGS }}

check_flags := '--all-targets'

alias c := check

# Check compilation of Rust code and catch common mistakes
# We cannot run --all-features because v1 and v2 are mutually exclusive features
# Create a list of features by excluding certain features 
clippy *FLAGS:
    #! /usr/bin/env bash
    set -euo pipefail

    FEATURES="$(cargo metadata --all-features --format-version 1 | \
        jq -r '
            [ ( .workspace_members | sort ) as $package_ids # Store workspace crate package IDs in `package_ids` array
            | .packages[] | select( IN(.id; $package_ids[]) ) | .features | keys[] ] | unique # Select all unique features from all workspace crates
            | del( .[] | select( any( . ; test("(([a-z_]+)_)?v2") ) ) ) # Exclude some features from features list
            | join(",") # Construct a comma-separated string of features for passing to `cargo`
    ')"

    set -x
    cargo clippy {{ check_flags }} --features "${FEATURES}"  {{ FLAGS }}
    set +x

clippy_v2 *FLAGS:
    #! /usr/bin/env bash
    set -euo pipefail

    FEATURES="$(cargo metadata --all-features --format-version 1 | \
        jq -r '
            [ ( .workspace_members | sort ) as $package_ids # Store workspace crate package IDs in `package_ids` array
            | .packages[] | select( IN(.id; $package_ids[]) ) | .features | keys[] ] | unique # Select all unique features from all workspace crates
            | del( .[] | select( any( . ; . == ("v1") ) ) ) # Exclude some features from features list
            | join(",") # Construct a comma-separated string of features for passing to `cargo`
    ')"

    set -x
    cargo clippy {{ check_flags }} --features "${FEATURES}"  {{ FLAGS }}
    set +x

check_v2 *FLAGS:
    #! /usr/bin/env bash
    set -euo pipefail

    FEATURES="$(cargo metadata --all-features --format-version 1 | \
        jq -r '
            [ ( .workspace_members | sort ) as $package_ids # Store workspace crate package IDs in `package_ids` array
            | .packages[] | select( IN(.id; $package_ids[]) ) | .features | keys[] ] | unique # Select all unique features from all workspace crates
            | del( .[] | select( any( . ; . == ("v1", "merchant_account_v2", "payment_v2") ) ) ) # Exclude some features from features list
            | join(",") # Construct a comma-separated string of features for passing to `cargo`
    ')"

    set -x
    cargo clippy {{ check_flags }} --features "${FEATURES}"  {{ FLAGS }}
    set +x

check *FLAGS:
    #! /usr/bin/env bash
    set -euo pipefail

    FEATURES="$(cargo metadata --all-features --format-version 1 | \
        jq -r '
            [ ( .workspace_members | sort ) as $package_ids # Store workspace crate package IDs in `package_ids` array
            | .packages[] | select( IN(.id; $package_ids[]) ) | .features | keys[] ] | unique # Select all unique features from all workspace crates
            | del( .[] | select( any( . ; test("(([a-z_]+)_)?v2") ) ) ) # Exclude some features from features list
            | join(",") # Construct a comma-separated string of features for passing to `cargo`
    ')"

    set -x
    cargo clippy {{ check_flags }} --features "${FEATURES}"  {{ FLAGS }}
    set +x

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

# Check compilation of v2 feature on base dependencies
v2_intermediate_features := "merchant_account_v2,payment_v2,customer_v2"
hack_v2:
    cargo hack clippy --feature-powerset --ignore-unknown-features --at-least-one-of "v2 " --include-features "v2" --include-features {{ v2_intermediate_features }} --package "hyperswitch_domain_models" --package "diesel_models" --package "api_models"
    cargo hack clippy --features "v2,payment_v2" -p storage_impl

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

ci_hack:
    scripts/ci-checks.sh


