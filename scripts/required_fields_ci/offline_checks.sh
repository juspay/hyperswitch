#!/usr/bin/env bash
# Called only after dependency provisioning, inside a network namespace without external routes.
set -euo pipefail
python3 -m unittest discover -s scripts/required_fields_ci -p 'test_contracts.py' -v
python3 scripts/required_fields_contracts/check_lock.py
python3 -m unittest discover -s scripts/required_fields_contracts -p 'test_*.py' -v
cargo test --offline --locked --manifest-path scripts/required_fields_contracts/Cargo.toml --test isolated
cargo test --offline --locked --manifest-path scripts/required_fields_inventory_checks/Cargo.toml --lib
cargo test --offline --locked --manifest-path scripts/required_fields_inventory_checks/Cargo.toml --no-default-features --lib cli::tests
cargo test --offline --locked --manifest-path scripts/required_fields_inventory_checks/Cargo.toml --lib seed_export_independent_semantic_inputs -- --ignored
cargo test --offline --locked --manifest-path docs/required_fields_sync_contracts/sdk-v1/rust-probe/Cargo.toml --lib
cargo test --offline --locked -p payment_methods --features v1,required_fields_sync,storage_impl/default,hyperswitch_interfaces/v1 --lib required_fields_renderer
cargo test --offline --locked -p payment_methods --features v1,required_fields_sync,storage_impl/default,hyperswitch_interfaces/v1 --lib export_router_cli_profiles -- --ignored
cargo check --offline --locked -p payment_methods --no-default-features --features v1,payouts,required_fields_sync,storage_impl/default,hyperswitch_interfaces/v1 --lib
cargo build --offline --locked -p router --features required_fields_sync --bin router
python3 scripts/required_fields_ci/test_seed_cli.py --root "$RF_SEED_FIXTURE_ROOT" --exported "$RF_CLI_FIXTURE_ROOT" --router "$CARGO_TARGET_DIR/debug/router"
cargo check --offline --locked -p router --bin router
# Explicit source build guards; a dependency failure must not masquerade as writer/v2 refusal.
python3 scripts/required_fields_ci/check_build_guards.py --root "$RF_SEED_FIXTURE_ROOT"
git diff --exit-code
