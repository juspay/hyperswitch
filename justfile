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

# Same as `run_v2`, but builds/runs with LLVM source-based coverage (.profraw under target/coverage-profraw/).
# Prereqs: `rustup component add llvm-tools-preview` and `cargo install grcov`
# After you stop the server (Ctrl+C), run `just coverage_html` to generate HTML + lcov.info.
run_v2_llvm *FLAGS:
    #! /usr/bin/env bash
    set -euo pipefail

    ROOT="$(pwd)"
    mkdir -p "${ROOT}/target/coverage-profraw"
    export RUSTFLAGS="-Cinstrument-coverage"
    export LLVM_PROFILE_FILE="${ROOT}/target/coverage-profraw/router-%p-%m.profraw"

    FEATURES="$(cargo metadata --all-features --format-version 1 --no-deps | \
        jq -r '
            [ .packages[] | select(.name == "router") | .features | keys[]
            | select( any( . ; test("(([a-z_]+)_)?v2") ) ) ]
            | join(",")
    ')"

    set -x
    cargo run --package router --no-default-features --features "${FEATURES}" {{ FLAGS }}
    set +x

# Merge .profraw into HTML + lcov.info.
# NOTE: Do NOT pass the whole repo to grcov — it walks every file and can take 10+ minutes or look "stuck".
# We only pass dirs that actually contain *.profraw (from run_v2_llvm / demos).
coverage_html:
    #! /usr/bin/env bash
    set -euo pipefail
    ROOT="$(pwd)"
    # grcov needs llvm-profdata on PATH (ships with Rust's llvm-tools-preview)
    HOST="$(rustc -vV | sed -n 's/^host: //p')"
    SYSROOT="$(rustc --print sysroot)"
    export PATH="${SYSROOT}/lib/rustlib/${HOST}/bin:${HOME}/.cargo/bin:${PATH}"

    command -v grcov >/dev/null 2>&1 || { echo "Install grcov: cargo install grcov  (then ensure ~/.cargo/bin is on PATH)" >&2; exit 1; }
    if ! command -v llvm-profdata >/dev/null 2>&1; then
        echo "llvm-profdata not found (required by grcov)." >&2
        echo "Install:  rustup component add llvm-tools-preview" >&2
        echo "If you use a pinned toolchain:  rustup component add llvm-tools-preview --toolchain \$(rustup show active-toolchain | cut -d' ' -f1)" >&2
        exit 1
    fi

    GRCOV_PATHS=()
    for d in "${ROOT}/target/coverage-profraw" "${ROOT}/target/coverage-poc-demo"; do
        if [[ -d "$d" ]] && [[ -n "$(find "$d" -maxdepth 1 -name '*.profraw' 2>/dev/null | head -1)" ]]; then
            GRCOV_PATHS+=("$d")
        fi
    done

    if [[ ${#GRCOV_PATHS[@]} -eq 0 ]]; then
        echo "No .profraw files found under target/coverage-profraw/ or target/coverage-poc-demo/." >&2
        echo "Run:  just run_v2_llvm  →  hit the API  →  Ctrl+C  →  just coverage_html" >&2
        exit 1
    fi

    mkdir -p "${ROOT}/target/coverage-html"
    echo "Running grcov (HTML pass can take several minutes on this repo)…" >&2
    set -x
    grcov "${GRCOV_PATHS[@]}" \
        --source-dir "${ROOT}" \
        --binary-path "${ROOT}/target/debug" \
        --output-type html \
        --output-path "${ROOT}/target/coverage-html" \
        --keep-only "crates/*" \
        --ignore-not-existing
    grcov "${GRCOV_PATHS[@]}" \
        --source-dir "${ROOT}" \
        --binary-path "${ROOT}/target/debug" \
        --output-type lcov \
        --output-path "${ROOT}/lcov.info" \
        --keep-only "crates/*" \
        --ignore-not-existing
    set +x
    # grcov >= 0.8 writes HTML under <output-path>/html/index.html (not index.html at output root).
    if [[ -f "${ROOT}/target/coverage-html/html/index.html" ]]; then
        echo "Open in browser: ${ROOT}/target/coverage-html/html/index.html"
    elif [[ -f "${ROOT}/target/coverage-html/index.html" ]]; then
        echo "Open in browser: ${ROOT}/target/coverage-html/index.html"
    else
        echo "HTML report not found under target/coverage-html/ (check grcov stderr above)." >&2
        exit 1
    fi
    [[ -f "${ROOT}/lcov.info" ]] && echo "LCOV: ${ROOT}/lcov.info" || echo "Warning: lcov.info missing (second grcov step may have failed)." >&2

# POC phase 1: emit **d** only (lcov vs static flow JSON). Empty pl; no specs loop (see script docstring).
# Needs lcov.info from any pipeline. Example:  just coverage_flow_gap --targets-only --out coverage_flow_gap.json
coverage_flow_gap *ARGS:
    python3 scripts/coverage_flow_gap.py \
        --flow-json get_connector_with_networks.json \
        --lcov lcov.info \
        --repo-root . \
        {{ ARGS }}

# curl /health → just coverage_html → diff for scripts/path_flow_health.json (router must be up; see script header)
health_curl_coverage_diff:
    bash scripts/run_health_curl_coverage_diff.sh

# Build/run instrumented router, curl /health, stop (flush .profraw), coverage_html, diff + line-hit table (needs DB/Redis; see script header)
health_llvm_e2e:
    bash scripts/llvm_health_coverage_e2e.sh

# Chain + lcov → d. Simplest Cypress pair: scripts/path_flow_health.json + scripts/coverage_pl_cypress.health.json (see scripts/README_coverage_pathflow.md)
coverage_feedback_loop *ARGS:
    python3 scripts/coverage_feedback_loop.py \
        --chain-artifact get_connector_with_networks.json \
        --lcov lcov.info \
        --repo-root . \
        {{ ARGS }}

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
