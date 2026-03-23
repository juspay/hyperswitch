# Path-flow JSON + Cypress + LLVM diff

## Why `async fn` and `no_lcov_da_in_span`

Rust `async fn` is lowered to a state machine. With **LLVM source-based coverage** (`-Cinstrument-coverage`) and **grcov**, **`DA:` lines are often missing for the original `async fn` source span** in `lcov.info` (probes attach to generated closures/lines instead). That is **not** fixed by deleting `.profraw` alone.

For the shallow health POC, the router exposes a small **sync** helper (`shallow_health_body`) so the path-flow leaf matches a span that **does** get `DA:` entries. The public handler remains `async fn health`.

---

Two ready-made pairs:

## 1) Simplest (recommended to try first)

| Piece | File |
|--------|------|
| Path-flow artifact (leaf = `health`, `GET /health`) | `scripts/path_flow_health.json` |
| Cypress (single spec, one `cy.request` to `/health`) | `cypress-tests/cypress/e2e/spec/Misc/00000-HealthCheck.cy.js` |
| `pl` JSON | `scripts/coverage_pl_cypress.health.json` |

Run diff (lcov only, no Cypress):

```bash
python3 scripts/coverage_feedback_loop.py \
  --chain-artifact scripts/path_flow_health.json \
  --lcov lcov.info \
  --repo-root .
```

Run Cypress then diff (needs router up + `npm ci` in `cypress-tests`):

```bash
just coverage_feedback_loop -- \
  --chain-artifact scripts/path_flow_health.json \
  --pl-json scripts/coverage_pl_cypress.health.json \
  --allow-exec
```

**Without Cypress** (only `curl` → `/health`; same path-flow leaf):

```bash
just coverage_feedback_loop -- \
  --chain-artifact scripts/path_flow_health.json \
  --pl-json scripts/coverage_pl_health_curl.json \
  --allow-exec
```

After an **instrumented** router run + **`just coverage_html`**, `d` can show real `DA:` / hits for `crates/router/src/routes/health.rs` if the build matches.

### One-shot: start instrumented router → `curl /health` → stop → `lcov` + diff

Requires a working local stack (Postgres, Redis, valid `config/…`), same as `just run_v2_llvm`.

```bash
just health_llvm_e2e
# or:  ./scripts/llvm_health_coverage_e2e.sh
```

This builds with `-Cinstrument-coverage`, runs `target/debug/router` with `LLVM_PROFILE_FILE` under `target/coverage-profraw/`, waits for shallow health (default **`GET /v2/health`** — v2-only router; override with `HEALTH_PATH=/health` if you use v1), curls it, **SIGTERM**s the router (flush `.profraw`), runs **`just coverage_html`**, then **`coverage_feedback_loop`** with **`--print-line-hits`** (per-line table on stderr) plus a short stdout summary.

If the router is already built with the same v2 features + `-Cinstrument-coverage`: `SKIP_BUILD=1 ./scripts/llvm_health_coverage_e2e.sh`.

## 2) Payments / `get_connector_with_networks`

| Piece | File |
|--------|------|
| Path-flow artifact | `get_connector_with_networks.json` (repo root) |
| Cypress (v1 payments, heavier) | `scripts/coverage_pl_cypress.example.json` → `00004-NoThreeDSAutoCapture.cy.js` |

`get_connector_with_networks` is only hit for specific debit-routing + network paths; many payment tests exercise `/payments` without touching that leaf.

## Generating path-flow for another function

There is no generator in this POC set. Reuse the shape of `path_flow_health.json` or `get_connector_with_networks.json`: top-level `function` / `file` / `def_line`, `endpoints[]`, `flows[].chain[]` with one `role: "target"` step including accurate `source` for brace-based body span.
