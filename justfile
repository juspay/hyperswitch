# List available recipes
list:
    @just --list --justfile {{ source_file() }}

fmt_flags := '--all'

# Run formatter
fmt *FLAGS:
    cargo +nightly fmt {{ fmt_flags }} {{ FLAGS }}

check_flags := '--all-targets'
v2_lints:= '-D warnings -Aunused -Aclippy::todo -Aclippy::diverging_sub_expression'

alias c := check

# Check compilation of Rust code and catch common mistakes
# We cannot run --all-features because v1 and v2 are mutually exclusive features
# Create a list of features by excluding certain features
clippy *FLAGS:
    #! /usr/bin/env bash
    set -euo pipefail

    FEATURES="$(cargo metadata --all-features --format-version 1 --no-deps | \
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

    FEATURES="$(cargo metadata --all-features --format-version 1 --no-deps | \
        jq -r '
            [ ( .workspace_members | sort ) as $package_ids # Store workspace crate package IDs in `package_ids` array
            | .packages[] | select( IN(.id; $package_ids[]) ) | .features | keys[] ] | unique # Select all unique features from all workspace crates
            | del( .[] | select( . == ("default", "v1") ) ) # Exclude some features from features list
            | join(",") # Construct a comma-separated string of features for passing to `cargo`
    ')"

    set -x
    cargo clippy {{ check_flags }} --no-default-features --features "${FEATURES}" -- {{ v2_lints }} {{ FLAGS }}
    set +x

check_v2 *FLAGS:
    #! /usr/bin/env bash
    set -euo pipefail

    FEATURES="$(cargo metadata --all-features --format-version 1 --no-deps | \
        jq -r '
            [ ( .workspace_members | sort ) as $package_ids # Store workspace crate package IDs in `package_ids` array
            | .packages[] | select( IN(.id; $package_ids[]) ) | .features | keys[] ] | unique # Select all unique features from all workspace crates
            | del( .[] | select( . == ("default", "v1") ) ) # Exclude some features from features list
            | join(",") # Construct a comma-separated string of features for passing to `cargo`
    ')"

    set -x
    cargo check {{ check_flags }} --no-default-features --features "${FEATURES}" -- {{ FLAGS }}
    set +x

build_v2 *FLAGS:
    #! /usr/bin/env bash
    set -euo pipefail

    FEATURES="$(cargo metadata --all-features --format-version 1 --no-deps | \
        jq -r '
            [ .packages[] | select(.name == "router") | .features | keys[] # Obtain features of `router` package
            | select( any( . ; test("(([a-z_]+)_)?v2") ) ) ] # Select v2 features
            | join(",") # Construct a comma-separated string of features for passing to `cargo`
    ')"

    set -x
    cargo build --package router --bin router --no-default-features --features "${FEATURES}" {{ FLAGS }}
    set +x


run_v2:
    #! /usr/bin/env bash
    set -euo pipefail

    FEATURES="$(cargo metadata --all-features --format-version 1 --no-deps | \
        jq -r '
            [ .packages[] | select(.name == "router") | .features | keys[] # Obtain features of `router` package
            | select( any( . ; test("(([a-z_]+)_)?v2") ) ) ] # Select v2 features
            | join(",") # Construct a comma-separated string of features for passing to `cargo`
    ')"

    set -x
    cargo run --package router --no-default-features --features "${FEATURES}"
    set +x

check *FLAGS:
    #! /usr/bin/env bash
    set -euo pipefail

    FEATURES="$(cargo metadata --all-features --format-version 1 --no-deps | \
        jq -r '
            [ ( .workspace_members | sort ) as $package_ids # Store workspace crate package IDs in `package_ids` array
            | .packages[] | select( IN(.id; $package_ids[]) ) | .features | keys[] ] | unique # Select all unique features from all workspace crates
            | del( .[] | select( any( . ; test("(([a-z_]+)_)?v2") ) ) ) # Exclude some features from features list
            | join(",") # Construct a comma-separated string of features for passing to `cargo`
    ')"

    set -x
    cargo check {{ check_flags }} --features "${FEATURES}" {{ FLAGS }}
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

doc_flags := '--all-features --all-targets --exclude-features "v2"'

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
v2_compatible_migrations := source_directory() / 'v2_compatible_migrations'
v1_migration_dir := source_directory() / 'migrations'
resultant_dir := source_directory() / 'final-migrations'

# Copy migrations in {{dir_1}} and {{dir_2}} to a single directory {{resultant_dir}} after prefixing the subdirectories of {{dir_2}} with {{prefix}}
[private]
prefix_and_copy_migrations dir_1 dir_2 prefix resultant_dir:
    #! /usr/bin/env bash
    mkdir -p {{resultant_dir}}
    cp -r {{dir_1}}/* {{resultant_dir}}/ > /dev/null 2>&1

    # Prefix v2 migrations with {{prefix}}
    sh -c '
    for dir in "{{dir_2}}"/*; do
        if [ -d "${dir}" ]; then
            base_name=$(basename "${dir}")
            new_name="{{prefix}}${base_name}"
            cp -r "${dir}" "{{resultant_dir}}/${new_name}"
        fi
    done
    '
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
    just prefix_and_copy_migrations {{ v1_migration_dir }} {{ v2_compatible_migrations }} 8 {{ resultant_dir }}
    just prefix_and_copy_migrations {{ resultant_dir }} {{ v2_migration_dir }} 9 {{ resultant_dir }}
    just run_migration {{ operation }} {{ resultant_dir }} {{ v2_config_file_dir }} {{ database_url }} {{ args }} || EXIT_CODE=$?
    just delete_dir_if_exists
    exit $EXIT_CODE

# Run database migrations compatible with both v1 and v2
migrate_v2_compatible:
    #! /usr/bin/env bash
    set -euo pipefail

    EXIT_CODE=0
    just prefix_and_copy_migrations {{ v1_migration_dir }} {{ v2_compatible_migrations }} 8 {{ resultant_dir }}

    # Run the compatible migrations
    just run_migration run {{ resultant_dir }} {{ database_url }} || EXIT_CODE=$?

    just delete_dir_if_exists
    exit $EXIT_CODE

# Drop database if exists and then create a new 'hyperswitch_db' Database
resurrect database_name=db_name:
    psql -U postgres -c 'DROP DATABASE IF EXISTS  {{ database_name }}';
    psql -U postgres -c 'CREATE DATABASE {{ database_name }}';

ci_hack:
    scripts/ci-checks.sh

# API Validation Commands

# Run complete API validation locally (compare current HEAD with main)
api-validate:
    ./scripts/local-api-validation.sh

# Compare API schemas between two references (commits, tags, or branches)
api-diff from='origin/main' to='HEAD':
    ./scripts/local-api-validation.sh {{ from }} {{ to }}

# Simulate PR validation locally (same as CI)
api-validate-pr:
    ./scripts/local-api-validation.sh origin/main HEAD ./pr-validation-output

# Generate API schemas without validation
api-generate-schemas:
    @echo "Generating V1 schema..."
    cargo run -p openapi --features v1
    @if [ -f "api-reference/v1/openapi_spec_v1.json" ]; then \
        cp api-reference/v1/openapi_spec_v1.json openapi-v1-current.json; \
        echo "✅ V1 schema copied to openapi-v1-current.json"; \
    else \
        echo "❌ V1 schema file not found"; \
    fi
    @echo "Generating V2 schema..."
    cargo run -p openapi --features v2
    @if [ -f "api-reference/v2/openapi_spec_v2.json" ]; then \
        cp api-reference/v2/openapi_spec_v2.json openapi-v2-current.json; \
        echo "✅ V2 schema copied to openapi-v2-current.json"; \
    else \
        echo "❌ V2 schema file not found"; \
    fi

# Run only Spectral linting on current schemas
api-lint:
    @echo "Generating schemas for linting..."
    cargo run -p openapi --features v1
    cargo run -p openapi --features v2
    @echo "Running Spectral validation..."
    @if [ -f "api-reference/v1/openapi_spec_v1.json" ]; then \
        spectral lint api-reference/v1/openapi_spec_v1.json --ruleset .spectral-hyperswitch.yml || true; \
    fi
    @if [ -f "api-reference/v2/openapi_spec_v2.json" ]; then \
        spectral lint api-reference/v2/openapi_spec_v2.json --ruleset .spectral-hyperswitch.yml || true; \
    fi

# Quick check for breaking changes only (no linting)
api-breaking-changes from='origin/main' to='HEAD':
    #! /usr/bin/env bash
    set -euo pipefail
    
    echo "🔍 Checking for breaking changes between {{ from }} and {{ to }}..."
    
    # Generate temp schemas
    temp_dir=$(mktemp -d)
    trap "rm -rf $temp_dir" EXIT
    
    # Extract schemas from git (no checkout needed!)
    echo "Extracting {{ from }} schemas..."
    git show "{{ from }}:api-reference/v1/openapi_spec_v1.json" > "$temp_dir/from-v1.json" 2>/dev/null || echo "{}" > "$temp_dir/from-v1.json"
    git show "{{ from }}:api-reference/v2/openapi_spec_v2.json" > "$temp_dir/from-v2.json" 2>/dev/null || echo "{}" > "$temp_dir/from-v2.json"
    
    echo "Extracting {{ to }} schemas..."
    if [[ "{{ to }}" == "HEAD" ]]; then
        # For HEAD, generate current schemas if they don't exist in git
        if ! git show "HEAD:api-reference/v1/openapi_spec_v1.json" > "$temp_dir/to-v1.json" 2>/dev/null; then
            echo "Generating current V1 schema..."
            cargo run -p openapi --features v1 >/dev/null 2>&1
            cp api-reference/v1/openapi_spec_v1.json "$temp_dir/to-v1.json" 2>/dev/null || echo "{}" > "$temp_dir/to-v1.json"
        fi
        if ! git show "HEAD:api-reference/v2/openapi_spec_v2.json" > "$temp_dir/to-v2.json" 2>/dev/null; then
            echo "Generating current V2 schema..."
            cargo run -p openapi --features v2 >/dev/null 2>&1
            cp api-reference/v2/openapi_spec_v2.json "$temp_dir/to-v2.json" 2>/dev/null || echo "{}" > "$temp_dir/to-v2.json"
        fi
    else
        git show "{{ to }}:api-reference/v1/openapi_spec_v1.json" > "$temp_dir/to-v1.json" 2>/dev/null || echo "{}" > "$temp_dir/to-v1.json"
        git show "{{ to }}:api-reference/v2/openapi_spec_v2.json" > "$temp_dir/to-v2.json" 2>/dev/null || echo "{}" > "$temp_dir/to-v2.json"
    fi
    
    # Check for breaking changes
    echo "V1 API:"
    oasdiff breaking "$temp_dir/from-v1.json" "$temp_dir/to-v1.json" || echo "✅ No breaking changes in V1"
    echo ""
    echo "V2 API:" 
    oasdiff breaking "$temp_dir/from-v2.json" "$temp_dir/to-v2.json" || echo "✅ No breaking changes in V2"

# Install API validation dependencies
api-install-deps:
    @echo "Installing API validation dependencies..."
    @echo "1. Installing Spectral CLI..."
    npm install -g @stoplight/spectral-cli@6.11.0
    @echo "2. Installing oasdiff..."
    @if command -v brew &> /dev/null; then \
        brew install oasdiff; \
    else \
        echo "Please install oasdiff manually from: https://github.com/Tufin/oasdiff/releases"; \
    fi
    @echo "✅ Dependencies installed. Run 'just api-validate' to test."
