# List available recipes
list:
    @just --list --justfile {{ source_file() }}
fmt_flags := '--all'
# Run formatter
fmt *FLAGS:
    cargo +nightly fmt {{ fmt_flags }} {{ FLAGS }}
check_flags := '--all-features --all-targets'
# Check compilation of Rust code
check *FLAGS:
    cargo check {{ check_flags }} {{ FLAGS }}
alias c := check
# Check compilation of Rust code and catch common mistakes
clippy *FLAGS:
    cargo clippy {{ check_flags }} {{ FLAGS }}
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
doc_flags := '--all-features --all-targets'
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
hack_flags := '--workspace --each-feature --all-targets'
# Check compilation of each cargo feature
hack:
    cargo hack check {{ hack_flags }}

db_user := 'db_user'
db_password := 'db_pass'
db_host := 'localhost'
db_port := '5432'
db_name := 'hyperswitch_db'
default_db_url := ('postgresql://' + db_user + ':'+ db_password +'@'+ db_host +':'+ db_port / db_name)

v2_migration_dir := 'v2_migrations'
v1_migration_dir := 'migrations'
resultant_dir := 'final-migrations'
default_operation := 'run'
default_migration_params := ''
v1_config_file_dir := 'diesel.toml'
v2_config_file_dir := 'diesel_v2.toml'
v1_config_params := ('--config-file ' + v1_config_file_dir)
v2_config_params := ('--config-file ' + v2_config_file_dir)

# Copy v1 and v2 migrations to a single directory
[private]
copy_migrations:
    @mkdir {{resultant_dir}}
    @cp -r {{v1_migration_dir}}/* {{resultant_dir}}
    @cp -r {{v2_migration_dir}}/* {{resultant_dir}}
    @echo "Created {{resultant_dir}}"

# Delete the newly created directory
[private]
delete_dir_if_exists dir=resultant_dir:
    @if [ -d "{{dir}}" ]; then \
        rm -r "{{dir}}"; \
        echo "Directory deleted at {{dir}}"; \
    else \
        echo "Directory {{dir}} does not exist"; \
    fi

[private]
run_migration operation=default_operation migration_dir=v1_migration_dir config_file_dir=v1_config_file_dir url=default_db_url other_params=default_migration_params:
    diesel migration \
        --database-url '{{url}}' \
        {{ operation }} \
        --migration-dir '{{migration_dir}}' \
        --config-file '{{config_file_dir}}' \
        {{other_params}} ||  just delete_dir_if_exists 

# Run database migrations for v1
migrate operation='run' *args='': (run_migration operation v1_migration_dir v1_config_file_dir default_db_url args)

# Run database migrations for v2
migrate_v2 operation='run' *args='': copy_migrations (run_migration operation resultant_dir v2_config_file_dir default_db_url args) delete_dir_if_exists

# Drop database if exists and then create a new 'hyperswitch_db' Database
reserruct:
    psql -U postgres -c 'DROP DATABASE IF EXISTS  hyperswitch_db WITH (FORCE)';
    psql -U postgres -c 'CREATE DATABASE hyperswitch_db';
