# Hyperswitch AI Coding Agent Guidelines

## Project Overview

Hyperswitch is a composable, modular payment infrastructure ("Linux for Payments") written in Rust. It's a workspace-based monorepo with two main binaries: **router** (main payment flows) and **scheduler** (async jobs). The Control Center dashboard and Web SDK live in separate repos.

**Tech stack:** Rust 1.85+, Diesel ORM, PostgreSQL, Redis, Actix-Web, Docker, `just`/`cargo`/`make` for builds.

## Architecture & Structure

- **`crates/router`**: Core payment engine, API routes, connector integrations, business logic
- **`crates/scheduler`**: Producer (job scheduling) + Consumer (job execution) for async tasks
- **`crates/api_models`**: Request/response models for the router crate
- **`crates/diesel_models`**: DB models, schema, migrations
- **`crates/hyperswitch_connectors`**: Payment connector implementations
- **`crates/common_utils`**, **`crates/common_enums`**, **`crates/masking`**: Shared utilities, types, and PII protection
- **`connector-template/`**: Boilerplate for adding new payment connectors

**Dual API versions:** Hyperswitch uses `v1` and `v2` features. Most modules default to `v1`. V2 is opt-in via feature flags (`cfg(feature = "v2")`). **Always check for `#[cfg(feature = "v1")]` / `#[cfg(feature = "v2")]`** when modifying core payment flows in `crates/router/src/core/payments/operations/`.

## Build & Dev Workflow

### Core Commands

```bash
# Format code (required for pre-commit)
just fmt

# Lint v1 code
just clippy

# Lint v2 code
just clippy_v2

# Check compilation (v1 features)
just check

# Check compilation (v2 features)
just check_v2

# Build binaries
cargo build

# Run router locally
cargo run --bin router

# Build & run v2-only router
just run_v2

# Run tests
cargo test --all-features
cargo nextest run  # faster, but skips doctests

# Pre-commit checks
just precommit  # fmt + clippy + test
```

### Database

- **Migrations:** Use `just migrate` (v1) or `just migrate_v2` (v2). Migration directories: `migrations/`, `v2_compatible_migrations/`, `v2_migrations/`.
- **Resurrect DB:** `just resurrect <db_name>` drops and recreates the database.

### Code Coverage

See `generate_code_coverage.sh`. Run tests with coverage enabled if modifying critical flows.

## Connector Integration

**Key guide:** `add_connector.md` (1256 lines) — comprehensive walkthrough for adding a new payment connector.

### Scaffolding a Connector

```bash
cargo install cargo-generate  # if not installed
# Use connector-template/ to scaffold a new connector
```

### Important Conventions

1. **PII Protection**: All sensitive data MUST use `masking::Secret<T>` or `common_utils::pii::*` types.
   - Example: `Secret<String, common_utils::pii::Email>`, `masking::Secret<String>` for API keys
   - See `crates/masking/` and `common_utils::pii` module

2. **Error Handling**: Use `error_stack::ResultExt` for context. Return `CustomResult<T, errors::ConnectorError>`.

3. **Transformers**: Place request/response transformers in `crates/hyperswitch_connectors/src/connectors/{name}/transformers.rs`.

4. **Traits to Implement**:
   - `ConnectorCommon` (base URL, error handling, headers)
   - `ConnectorIntegration<Flow, Request, Response>` for each payment flow (Authorize, Capture, PSync, Void, etc.)
   - See `connector-template/mod.rs` for boilerplate

5. **Control Center Integration**: Update `ConnectorTypes.res`, `ConnectorUtils.res`, add connector icon to control-center repo.

6. **Tests**: Place connector tests in `crates/hyperswitch_connectors/src/connectors/{name}/test.rs` or `cypress-tests/`.

## Coding Conventions

- **Workspace dependencies**: Most crates inherit edition/license/version via `[workspace]`. See `Cargo.toml` root.
- **Lints**: `workspace.lints.rust` and `workspace.lints.clippy` enforce strict rules (e.g., `forbid unsafe_code`, `warn on expect_used/unwrap_used`).
- **Use nightly for formatting**: `cargo +nightly fmt`.
- **Feature flags**: Check Cargo.toml for optional features (e.g., `analytics`, `v1`, `v2`, `olap`, `oltp`).
- **Modules are public**: Most router modules expose `pub mod` interfaces in `crates/router/src/lib.rs`.

### Enum Consolidation Pattern

**Important:** Enums should be consolidated into `crates/common_enums/src/enums.rs` to avoid duplication across crates.

**Example - UserStatus Enum:**
- ❌ **Before**: Duplicated in `crates/api_models/src/user_role.rs` and `crates/diesel_models/src/enums.rs`
- ✅ **After**: Single source in `crates/common_enums/src/enums.rs`, re-exported via `diesel_exports` module

**Pattern to follow:**
1. **Define once in `common_enums`**: Add enum to `crates/common_enums/src/enums.rs` with diesel attributes:
   ```rust
   #[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
   #[diesel_enum(storage_type = "db_enum")]
   #[serde(rename_all = "snake_case")]
   pub enum UserStatus {
       Active,
       #[default]
       InvitationSent,
   }
   ```

2. **Export for diesel**: Add to `diesel_exports` module in same file:
   ```rust
   pub mod diesel_exports {
       pub use super::DbUserStatus as UserStatus;
   }
   ```

3. **Import in other crates**: Use `common_enums::UserStatus` instead of defining locally
   - In `api_models`: Remove local definition, import from `common_enums`
   - In `diesel_models`: Use `enums::UserStatus` (re-exported from `common_enums`)
   - In `router`: Import via `common_enums::enums::UserStatus`

**Why this matters:**
- Prevents type mismatches across crate boundaries
- Single source of truth for database enums
- Easier to maintain and extend enum variants
- Consistent serialization/deserialization behavior

## Testing

- **Unit tests**: `#[cfg(test)]` modules inline or in `tests/` subdirectories.
- **Integration tests**: Cypress tests in `cypress-tests/` and `cypress-tests-v2/`.
- **Connector tests**: Mock sandbox credentials in `crates/test_utils/`.

## Monitoring & Observability

- OpenTelemetry (OTLP) for traces/metrics
- Prometheus, Loki, Tempo, Grafana stack (see `config/otel-collector.yaml`, `config/prometheus.yaml`)
- Logs: Use `router_env::logger` and `tracing` macros. Annotate functions with `#[instrument(skip_all, fields(flow = ?Flow::*))]`.

## Common Pitfalls

1. **Forgetting to run migrations** after schema changes → `just migrate` or `just migrate_v2`
2. **Not wrapping secrets** in `masking::Secret` → PCI/PII compliance failure
3. **Ignoring v1/v2 feature gates** → code may not compile in both modes
4. **Skipping `just precommit`** → CI will fail on formatting/clippy
5. **Not updating Control Center** after adding a connector → UI won't show the new connector

## Resources

- **Contributing guide:** `docs/CONTRIBUTING.md`
- **Architecture doc:** `docs/architecture.md`
- **Add connector guide:** `add_connector.md`
- **Local setup:** `docs/try_local_system.md`, `docker-compose.yml`
- **Slack:** [Hyperswitch Slack](https://inviter.co/hyperswitch-slack)

## Quick References

- **Router entry point:** `crates/router/src/bin/router.rs`
- **Payment flows:** `crates/router/src/core/payments/`
- **Connector implementations:** `crates/hyperswitch_connectors/src/connectors/`
- **API models:** `crates/api_models/src/`
- **DB models:** `crates/diesel_models/src/`

---

When in doubt, consult `README.md`, `add_connector.md`, and `docs/CONTRIBUTING.md`. For weekly office hours, check the #general Slack channel (Thursdays 8 AM PT).
