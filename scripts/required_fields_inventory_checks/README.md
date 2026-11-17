# Required-fields inventory local checks

This standalone crate imports the actual feature-gated router inventory files and Step 3
artifact types by path. It has no substitute implementation, AppState, CLI, credentials,
server target or hosted connection. Its 162 registry dependency identities/checksums match
H's `Cargo.lock`. The root manifest and lock are unchanged.

Inspect commands before running. Register a new disposable filesystem directory with an
owner/purpose marker and use it as `CARGO_TARGET_DIR`; verify ownership before cleanup.
Set `CARGO_NET_OFFLINE=true`, clear `RUSTC_WRAPPER`/`RUSTC_WORKSPACE_WRAPPER`, and use two
build jobs with incremental/debug output disabled. Then run from H:

```sh
cargo test --offline --locked --manifest-path scripts/required_fields_inventory_checks/Cargo.toml --lib
cargo check --offline --locked --manifest-path scripts/required_fields_inventory_checks/Cargo.toml --lib
cargo clippy --offline --locked --manifest-path scripts/required_fields_inventory_checks/Cargo.toml --lib --tests -- -D warnings
```

The current suite has 18 existing foundation tests, 23 inventory tests, five adapter tests
and one request comparison covering seven actual SDK 0.119.0 builders. Inventory responses
are independently written synthetic raw fixtures. The SDK connector captures requests in
memory; its placeholder responses are used only to exercise request serialization, never
as server evidence. The production HTTP adapter refuses synthetic certificates before
dispatch. Tests open no listener and use no DNS/network connection.

These checks do not establish server pagination, complete permission scope, real transport,
or router feature/release compatibility. See [Step 5 evidence](../../docs/required_fields_sync_contracts/inventory-v1/README.md)
and its [executed checks](../../docs/required_fields_sync_contracts/inventory-v1/checks.md).
