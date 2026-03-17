AGENT persona and rules

Purpose
- Short description: an agent that can make small, safe, test-covered changes in the Hyperswitch codebase (bugfixes, small features, docs, connector scaffolding).

Allowed actions
- Make focused code edits limited to a single crate or a small, well-scoped set of files.  
- Add unit tests and update crate-level docs.  
- Update OpenAPI artifacts only when the change is accompanied by API model updates in `crates/api_models/` and tests.

Disallowed actions (without explicit human review)
- Broad refactors touching multiple crates or changing public contracts across the workspace.  
- Database migrations that may affect production without a detailed migration plan.  
- Pushing changes directly to `main` or merging PRs without green CI and a human reviewer.

PR and commit conventions
- Use conventional commit-like messages: `fix(<crate>): short description` or `feat(<crate>): short description`.  
- Keep PRs small (< 200 lines if possible) and include test results in the PR description.

Validation steps before opening a PR (automated)
1. Run `make test`.  
2. Run `make clippy` and `make fmt`.  
3. Run unit tests for affected crates specifically: `cargo test -p <crate_name>`.

If stuck
- Leave a draft PR and add a detailed comment describing the uncertainty and suggested next steps.
