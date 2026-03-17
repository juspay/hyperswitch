# Copilot instructions for working on Hyperswitch

Purpose: make AI coding agents (Copilot-style assistants) productive quickly by listing the repository's architecture, workflows, conventions, and concrete commands or file locations an automated agent should consult or use.

- Big picture (high level)
  - Hyperswitch is a modular payments platform implemented as a Rust workspace (`Cargo.toml` at repo root). Key runtime components: `router` (API/orchestrator), `drainer` (drain/archival), and `scheduler` (producer/consumer flows).
  - The `router` is the main crate (see `crates/router/README.md` and `crates/router/src/*`). Connectors live under the connector directories (e.g., `crates/*` and `router/src/connector/*`). Core orchestration lives under `router/src/core` and HTTP endpoints under `router/src/routes` (Actix Web).
  - Config-driven deployments: configs live in `config/deployments/*.toml`. The router binary accepts `--config-path` (see `config/deployments/README.md`) and health is exposed at `/health`.

- Where to read first (quick links for an agent)
  - Repository overview: `README.md` (root) — contains quickstart and architecture diagrams.
  - Router crate overview: `crates/router/README.md` and `crates/router/src/` — core flow, routes, connectors.
  - Deployment config and examples: `config/deployments/README.md`, `config/deployments/sandbox.toml` (environment defaults) and `env_specific.toml` (sensitive overrides).
  - Workspace settings and enforced lints: `Cargo.toml` (workspace), `cargo`/`justfile`/`Makefile` (build/test recipes).

- Concrete developer workflows (commands an agent can use)
  - Local full dev via Docker (recommended): run `scripts/setup.sh` (root `README.md` quickstart). This boots Compose-based local environment.
  - Quick local build: `make build` or `cargo build` (Makefile target `build`).
  - Run in-place (dev): `just run` (uses workspace feature selection). To run the v2 router feature set use `just run_v2` or `just build_v2` (see `justfile`).
  - Tests: `make test` or `cargo test --all-features`. For nextest: `just nextest` / `cargo nextest run`.
  - Lint & fmt: `make fmt` / `cargo +nightly fmt --all` and `make clippy` or `just clippy` (the repository uses custom feature selection for clippy — prefer the `justfile` wrappers).
  - Pre-commit check: `make precommit` or `just precommit` which runs fmt + clippy + tests.
  - DB migrations: `just migrate` (v1), `just migrate_v2` (v2-compatible flows). See `justfile` for the prefix-and-copy migration helpers.

- Important project-specific conventions
  - v1 vs v2 feature sets are mostly mutually exclusive. The `justfile` contains helpers to assemble feature lists; don't assume `--all-features` works for local full builds.
  - Connectors: connector-specific code (gateway transformations and adapters) belongs in connector subfolders; keep connector logic isolated and small — core orchestrator code belongs under `core/`.
  - DB layer: Diesel is used for storage models (see `crates/diesel_models` and `router/src/types/storage`). Migration directories: `migrations`, `v2_migrations`, `v2_compatible_migrations`.
  - Scheduler flows: `SCHEDULER_FLOW` environment variable controls `producer` vs `consumer` behavior when running scheduler images (see `config/deployments/README.md`).

- Integration & deployment notes (what an agent should know)
  - Docker images used in `docker-compose.yml` reference docker.juspay.io images; the router/scheduler/drainer binaries expect `--config-path /local/config/deployments/<env>_release.toml`.
  - Hosted sandbox exists (see root `README.md`) and `config/deployments/sandbox.toml` contains recommended defaults; merging with `env_specific.toml` produces release configs.

- Examples agents should include when making changes
  - When adding a connector: update `crates/*` connector folder, add mapping in `router` connector registry (search `connector::` in `crates/router/src`), and add README/docs in `connector-template/`.
  - When changing config keys: update `config/config.example.toml` and `config/deployments/*` and document expected `env_specific.toml` entries.

- Quick reference (files to read/modify for common tasks)
  - Build & workspace: `Makefile`, `justfile`, `Cargo.toml`
  - Router internals: `crates/router/src/` and `crates/router/README.md`
  - Connectors: `crates/hyperswitch_connectors` or `router/src/connector/` (search for specific connector folders)
  - Migrations: `migrations/`, `v2_migrations/`, `v2_compatible_migrations/`
  - Deployment configs: `config/deployments/*.toml` and `config/env_specific.toml`

If anything here is unclear or you want additional examples (for example, a short step-by-step example of adding a connector or running a specific integration test), tell me which area and I will expand or adjust the instructions.
