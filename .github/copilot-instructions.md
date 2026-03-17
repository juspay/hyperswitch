## Quick context for AI coding agents

Hyperswitch is a Rust Cargo workspace implementing a modular payments platform. Most code is under `crates/` (examples: `crates/router`, `crates/hyperswitch_connectors`, `crates/api_models`, `crates/common_types`). This file tells an automated agent how to be useful and safe when making changes.

Key points
- Build & test: use `make build`, `make test` (workspace), `make clippy`, `make fmt`, and `make nextest` for nextest runs.
- Local full-stack: run `scripts/setup.sh` to start the full local environment (docker-compose). Control Center UI and monitoring stacks are available via this flow.
- Public APIs: API models live in `crates/api_models/` and OpenAPI artifacts in `api-reference/`. If you change shapes, update OpenAPI and ensure CI's OpenAPI validation passes.
- Connectors: use `connector-template/` as a scaffold and implement connectors under `crates/hyperswitch_connectors/`. Follow interfaces in `crates/hyperswitch_interfaces`.
- DB & migrations: Diesel models are in `crates/diesel_models/`; migrations live in `migrations/` and are validated by CI (`.github/scripts/validate_migrations.sh`).

When making edits (required checklist)
1. Create small, focused changes (one concern per PR).  
2. Add or update unit tests for the crate you changed. Run `make test`.  
3. Run `make clippy` and `make fmt` locally. CI will run `-D warnings` for clippy.  
4. If you change API shapes, update `api-reference/` and re-run OpenAPI validation.  
5. If you change DB schema, add a Diesel migration under `migrations/`.

Where to start for common tasks (examples)
- Add a connector: copy `connector-template/`, implement trait methods referencing `crates/hyperswitch_interfaces`, add tests in the connector crate, and run `make test`.  
- Change routing/flow: inspect `crates/router/src/` (entrypoints in `lib.rs`), update logic, and add unit tests in `crates/router`.
- Update shared models: update `crates/api_models` and any dependent crates; run workspace tests `make test`.

CI and workflows
- CI lives in `.github/workflows/` (unit tests, wasm checks, cypress runs, OpenAPI validation). PRs must pass CI before merge.

If uncertain
- Search for symbols across `crates/*`. Read crate-level `README.md` files for crate-specific notes. Ask for a minimal PR suggestion if unsure (e.g., "I will make a focused PR that changes X and adds tests — OK?").

— end
