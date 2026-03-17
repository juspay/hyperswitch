# AGENT guidance for automated code agents

Purpose: provide operational rules, guardrails, and quick checklists for automated agents (Copilot-style assistants) working in this repository. This file complements `.github/copilot-instructions.md` by specifying what an agent may safely change, required verification steps, and when to escalate to a human reviewer.

1. Scope & intent

- Allowed: small focused edits (docs, READMEs), tests, formatting fixes, adding small helper scripts, non-sensitive config examples, and low-risk plumbing changes (CI job metadata, minor refactors limited to a crate).
- Not allowed: any change that touches production secrets, infra deployment manifests, credentials, direct pushes to protected branches, database credentials in `env_specific.toml`, or release/publishing flows without explicit human sign-off.

2. Allowed actions (examples)

- Edit or add documentation: `README.md`, `crates/*/README.md`, `config/deployments/*` examples.
- Fix formatting and run formatters: `make fmt` or `cargo +nightly fmt --all`.
- Run and fix failing unit tests locally and add small unit tests (keep runs fast): `make test` or `cargo test -p <crate>`.
- Add lightweight utility scripts in `scripts/` or small CI job fixes (non-destructive).

3. Forbidden actions

- Do not add or commit secrets (API keys, DB passwords, private keys). Files like `config/deployments/env_specific.toml` must not be populated with real secrets.
- Do not trigger production deploys or change cloud infra manifests (CDK/Helm) without human approval.
- Do not rework large refactors across many crates automatically — escalate instead.

4. Quality gates (must run before committing)

- Formatting: `make fmt` or `cargo +nightly fmt --all`.
- Lint: `just clippy` or `make clippy`. Use `just` wrappers to respect v1/v2 feature selection.
- Tests: `make test` (fast) and targeted `cargo test -p <crate>`; for broader validation use `just nextest` or `cargo nextest run` if available in CI.
- Documentation: when touching public APIs or behavior, update `crates/router/README.md` or root `README.md` and run `cargo doc --package router` to ensure it builds.

5. Project-specific gotchas

- v1 vs v2 features are mutually exclusive. Use `just` helpers (`just run_v2`, `just build_v2`, `just clippy_v2`) to assemble the correct feature list — do not assume `--all-features` will work.
- Database migrations are assembled via `just migrate` / `just migrate_v2` which prefix and copy migration trees (`migrations/`, `v2_migrations/`, `v2_compatible_migrations`). Never run migrations against production DBs.
- Connectors live under `crates/*` (see `crates/hyperswitch_connectors`) and `crates/router/src/connector/`; registry mappings must be updated where connector discovery/registration is implemented.
- Scheduler flow behavior depends on `SCHEDULER_FLOW` env var (producer vs consumer). See `config/deployments/README.md` and `docker-compose.yml` snippets.

6. Testing & verification checklist (template)

- Run formatters: `make fmt`
- Run lints: `just clippy`
- Run unit tests relevant to change: `cargo test -p <crate>`
- If changing router internals: `cargo doc --package router`
- If touching migrations: run `just migrate_v2_compatible` against a disposable local DB

7. When to escalate to a human

- Ambiguous spec or missing information about required behavior.
- Changes that require secrets, infra access, or updating `env_specific.toml` with real credentials.
- Large refactors spanning multiple crates (>3 crates) or touching migration history.
- Behavioural changes to routing, retries, reconciliation, or billing logic that could impact production correctness.

8. Minimal PR checklist for agents (to include in PR description)

- Short summary of change (1–2 lines)
- Files changed (list)
- Commands run locally (fmt, clippy, tests) and their exit statuses
- Tests added/modified (list tests)
- Risks & rollback notes (how a human can revert safely)

9. Action plan template (for non-trivial tasks)

- Goal: concise statement of desired result
- Inputs: files/configs/feature flags to modify
- Outputs: files, tests, docs to be produced
- Verification: exact commands to run and expected outcomes
- Escalation: who to ask and what to provide

10. References

- `.github/copilot-instructions.md` — orientation + commands
- `README.md` (root) — quickstart & architecture
- `justfile`, `Makefile`, `Cargo.toml` — build/lint/test recipes
- `config/deployments/README.md`, `config/deployments/*.toml`
- `crates/router/README.md` and `crates/*` directories

Security note: Agents must never exfiltrate or store secrets. If a test requires credentials, create a template file such as `config/deployments/env_template.toml` and ask a human to populate it.

If you'd like, I can commit this file now. Otherwise, tell me any specific rules you'd like added or tightened (for example: stricter test thresholds, CI-only ops, or required human approvers for certain paths).
