# cost-router

Cost-aware payment routing for [Hyperswitch](https://github.com/juspay/hyperswitch), built as a NestJS/TypeScript layer **in front of**
an unmodified Hyperswitch. For each payment it ranks the configured connectors by expected cost (processing fee **and** reliability), sends
that order to Hyperswitch as a `routing: {type: "priority", data: [...]}` override, records per-attempt outcomes, and keeps a decision trace.

No Hyperswitch source, config, schema or migration is modified: everything lives in `cost-router/` (plus `.claude/` and `DECISIONS.md`).
Why a layer and not Rust inside the router: see `../DECISIONS.md`.

```
client ──POST /payments──▶ cost-router ──POST /payments {routing: priority [...]}──▶ Hyperswitch ──▶ connectors
                              │  decide(): fee + reliability → ranked list            │
                              │◀── GET /payments/{id}?expand_attempts=true ───────────┘   (per-attempt outcomes)
                              └─ stats, decision trace, idempotency record (in memory)
```

## Prerequisites
- **Docker Desktop** (Windows: WSL2 backend) or Docker Engine with the compose plugin. Tested on Windows 10 with 8 GB RAM and WSL2 capped at 4 GB.
- **Node.js >= 18** (tested with v24) and npm.
- **bash** (Git Bash on Windows) for `scripts/up.sh`.
- **Python 3** only for the optional `scripts/hyperswitch-probe.sh`; the demo, tests and load test need only Node.
- Internet access for the first image pull (the stack images are several GB).
- Free ports `5432`, `6379`, `8080` and `8081` (a leftover Docker stack on these gives "port is already allocated"; stop it with
  `docker compose -p <name> down`). The demo also uses `3110`.

## Quick start
Any OS, including Windows PowerShell or cmd (on Windows, Git for Windows must be installed; Git Bash is found automatically):
```bash
git clone --branch feature/cost-aware-routing https://github.com/nileshvishwakarma51/hyperswitch.git
cd hyperswitch/cost-router
npm install
npm run up          # Hyperswitch stack (prebuilt images, dummy connectors); superposition seeding takes ~3 min
npm run build
npm test            # unit and adapter tests, no Docker needed
npm run demo        # end-to-end demo against the stack, one PASS/FAIL line per check
npm run down        # stop the stack (keeps the database volume)
```
What to expect:
- `npm run up` takes about 4 min once images are downloaded (on the first run, downloads of several GB dominate; allow longer on a slow
  connection). It ends with `Container <folder>-hyperswitch-server-1 Started` and then `health is good`.
- The lines `Failed to apply patch: error applying hunk #5` and `migration_runner-1 exited with code 1` are expected and harmless: they
  come from upstream's `schema.rs` patch step after all migrations have been applied.
- A long pause on `superposition-init` is its seeding (about 3 min).
- Check the stack: `curl localhost:8080/health` returns `health is good`.
- `npm test` prints an `ERROR [cost-router] routing failed for boom, override omitted: stats backend down` line. That is the intentional
  failure-mode test (the router drops its override and Hyperswitch routes normally), not a failure. It ends with 152 passed.
- `npm run demo` ends with `ALL CHECKS PASSED` (18 checks).
The same from bash: `bash cost-router/scripts/up.sh`, then `bash scripts/demo.sh` and `bash scripts/down.sh` inside `cost-router/`.
Windows PowerShell wrappers: `powershell -ExecutionPolicy Bypass -File cost-router\scripts\up.ps1` (also `demo.ps1`, `down.ps1`).
`make demo` runs the whole chain (stack, install, build, demo); it is untested on the dev machine (no `make`). The first run is dominated by
image downloads, not by our code.

### What the demo shows (`scripts/demo.sh`, real Hyperswitch, dummy connectors)
It creates a fresh merchant with two connectors, **fauxpay** (cheap, flaky: fails on the test card `4000000000009995`) and **stripe_test**
(dearer, reliable), then starts cost-router itself with fresh in-memory stats for each scenario.

| Step | Scenario | What you see |
|---|---|---|
| A1 | auto-retry on, cold start | no reliability data: cheapest wins, order `[fauxpay, stripe_test]` |
| A2 | auto-retry on, flaky card | fauxpay fails, Hyperswitch retries on the next connector in our list, payment `succeeded` on stripe_test, both attempts recorded |
| A3 | same, next payment | order flips to `[stripe_test, fauxpay]`: stripe_test costs more (fee 320 vs 110) but its expected cost is lower (score 740 vs 866) |
| trace | `GET /payments/{id}/routing-trace` and `/v1/...` | inputs, per-candidate fee / `s_hat` / score, `L_eff`, break-even penalty, requested order vs what Hyperswitch actually executed |
| replay | same `Idempotency-Key` | same body returns the stored result (`replayed: true`); different body returns 409 |
| B1 | **auto-retry off**, reliability floor 0.85 | fauxpay fails; there is no failover, the payment fails after one attempt |
| B2 | same | every connector is below the floor: `reliability_floor_unmet`, **no override sent**, Hyperswitch routes normally |

The demo's `failure_penalty_pct=20`, `min_sample_size=3` and floor `0.85` are **illustrative business assumptions**, not recommendations.

## Troubleshooting
- **`port is already allocated`**: another stack holds 5432, 6379, 8080 or 8081. Stop it (`docker compose -p <name> down`), then `npm run up` again.
- **`curl localhost:8080/health` is refused**: the stack is still starting or seeding; wait about 3-4 min after `npm run up` starts.
- **Windows: `bash` fails with a WSL error**: use `npm run up` / `npm run demo` (they find Git Bash) instead of typing `bash` in PowerShell.
- **Clean restart**: `npm run down -- -v` also deletes the database volume.
- **No `make` on Windows**: use the Quick start commands; `make demo` is for Linux/macOS.

## Using it by hand
```bash
cp config.example.json config.json      # edit candidates[].merchant_connector_id, fees, failure_penalty_pct; no API key in the file
export HYPERSWITCH_API_KEY=...          # the merchant api-key; never put it in the file or in git
COST_ROUTER_CONFIG=config.json npm start        # listens on the configured port (3100 in the example)
curl -X POST localhost:3100/payments -H 'content-type: application/json' -H 'Idempotency-Key: pay_001' \
     -d '{"amount":10000,"currency":"USD","confirm":true, ...the rest of your Hyperswitch create body...}'
curl localhost:3100/payments/pay_001/routing-trace
```
`scripts/hyperswitch-probe.sh` creates a merchant, connectors and a profile on the stack and prints the connector ids to put into `config.json`.

A stable payment id is required: `Idempotency-Key` header or body `payment_id` (it becomes the Hyperswitch `payment_id`). Replaying the
same id with the same request returns the stored decision and response without forwarding again; the same id with a different request body
returns **409 Conflict** (a sha256 fingerprint of the canonical body, excluding any caller `routing`, is stored, never the body itself).
A caller-supplied `routing` is stripped and never forwarded. Optional hints for fee matching are this service's own headers
`x-cost-router-bin-class` and `x-cost-router-card-type` (they are not Hyperswitch fields, are not forwarded, and must never carry a card number).
Bad config stops startup; a runtime engine error forwards the payment **without** an override, so a payment never fails because of this layer.

## Trace
`POST /payments` returns `{replayed, routing, payment}`; `GET /payments/{id}/routing-trace` (also served as `GET /v1/payments/{id}/routing-trace`,
the path named in the assignment) returns the same `routing` object.
- `routing.decision`: the **requested** routing. `ordered`, `fallbackToHyperswitch`, `fallbackReason`, and `trace` with the inputs (currency, minor-unit amount, hints),
  `failurePenaltyPct`, `lEffRaw`, `lEff`, `lEffClamped`, `breakEvenPenaltyPct`, and per candidate: `fee`, `sHat`, observed attempt counts per outcome class,
  success and failure score terms, `score`, `rank`, `status` and `statusReason`, plus a one-line `summary`.
- `routing.execution`: what Hyperswitch **actually did**. `requestedOrder`, `finalConnector`, `finalStatus`, `attempts`, `usedOutsideRequested`,
  `finalIsFirstRequested`, `notes`. Hyperswitch does not guarantee the order of `attempts[]`, so `attemptOrderEstablished` is always `false`
  and attempts are listed alphabetically, never by time.
- No card data is stored: only the engine trace and a request hash.

## Configuration (`routing` section; validated at startup with zod, unknown keys rejected)
| Key | Required | Meaning |
|---|---|---|
| `fee_table.<connector>[]` | yes | Rules: `match{currency, bin_class?, card_type?, min_amount_minor?, max_amount_minor?}`, `bps` (0..10000), `fixed`, `min` (default 0), `cap?`. Integer minor units. Fee = `clamp(round_half_up(amount*bps/10000)+fixed, min, cap)`. Most specific rule wins; duplicate/ambiguous overlapping rules are rejected. Zero fee is valid. No matching rule = connector is unpriced and ranked last. If no candidate has usable pricing, no override is sent. |
| `failure_penalty_pct` | **yes, no default** | Net expected business cost of a failed first attempt after fallback, as a percent of the payment amount. Derive it from the merchant's own margin and checkout-abandonment data. Plain decimal, finite, within 0..100 inclusive. 0 is allowed: `L_eff` is then set by the clamp alone (the highest priced candidate fee). It has no default on purpose: any built-in number would be invented and would silently decide the ranking. The example file's `20` is a placeholder. The trace shows the configured value, the raw `L_eff`, the clamped `L_eff` and a clamped flag; the configured value is never mutated. |
| `min_sample_size` (k) | yes | Shrinkage strength: `s_hat = (successes + k*prior)/(n + k)`, with one common prior: the pooled success rate over all candidate connectors that have data (1 when none has). |
| `window_minutes` | yes | Rolling window for stats. |
| `min_success_rate` | **yes** | Absolute floor on `s_hat`, in [0, 1]. 0 means no connector is excluded (an explicit "no floor" instead of a silent default). The example file's `0.85` is an illustrative assumption: pick it from the merchant's own authorization-rate data. Connectors under the floor are dropped when at least one priced connector passes. If **every** priced connector is under it, no override is sent (`fallbackToHyperswitch`, `reliability_floor_unmet`); it never ranks by success rate. Because `s_hat` is shrunk, a connector needs some history to clear a high floor, and a few early failures with no successes anywhere put every connector below it. |
| `issuer_decline_codes` | no (default `[]`) | Error codes treated as issuer declines (excluded from the denominator). **With the empty default every `failure` / `authorization_failed` counts as a technical failure, so a connector that receives more issuer declines can be penalised unfairly.** See `docs/ATTEMPT_OUTCOMES.md`. |
| `attempt_dedupe_ttl_minutes` | no (default 2 x `window_minutes`) | How long a counted `(payment_id, attempt_id)` is remembered. Must be >= `window_minutes`. Implementation limit, not a business tunable. |
| `attempt_dedupe_max_entries` | no (default 100000) | Cap on remembered attempts; oldest evicted first. Implementation limit. |
| `decision_ttl_minutes` | no (default 1440) | How long a payment's decision/response is kept for idempotent replay and `routing-trace`. After it, the id behaves like a new payment and the trace returns 404. Implementation limit. |
| `decision_max_entries` | no (default 100000) | Cap on stored decisions; oldest evicted first, never an in-flight one. Implementation limit. |

Top level: `port`, `hyperswitch.base_url`, `hyperswitch.api_key` (optional, not recommended), `state_file` (optional, see below), `candidates[]` (`{connector, merchant_connector_id}`; the id is required because Hyperswitch ignores list entries without it).

### Runtime settings: file vs environment (environment wins)
| Setting | Environment variable | File key | Notes |
|---|---|---|---|
| Config file path | `COST_ROUTER_CONFIG` (default `config.json`) | n/a | |
| Hyperswitch API key | `HYPERSWITCH_API_KEY` | `hyperswitch.api_key` | Required from one of the two. Never logged or put in errors/traces. |
| Hyperswitch base URL | `HYPERSWITCH_BASE_URL` | `hyperswitch.base_url` | Required from one of the two. |
| Port | `PORT` | `port` | Required from one of the two. |
| State file | `COST_ROUTER_STATE_FILE` | `state_file` | Optional. Unset = in memory only. |

Startup validates everything at once and fails with a single error listing every missing or invalid item (including a missing or non-JSON
config file); required routing fields keep their "required, no default" messages.

## Metrics (Prometheus)
`GET /metrics` serves the Prometheus text format (scrape it with any Prometheus). Labels are only connector names, currencies and fixed
enums, never payment ids.

| Metric | Meaning |
|---|---|
| `cost_router_routing_decisions_total{override, fallback_reason}` | decisions: priority override sent, or omitted (floor unmet, no pricing, engine error) |
| `cost_router_connector_first_choice_total{connector, currency}` | how often each connector was ranked first |
| `cost_router_candidate_evaluations_total{connector, status}` | per-connector result in each decision (eligible, below_floor, unpriced) |
| `cost_router_connector_success_estimate{connector, currency}` | latest `s_hat` per connector |
| `cost_router_attempts_total{connector, outcome}` | attempts read back from Hyperswitch, by outcome class |
| `cost_router_decision_duration_seconds` | time inside the routing engine |
| `cost_router_request_duration_seconds{result}` | `POST /payments` end to end |

Node process metrics (`cost_router_process_*`, `cost_router_nodejs_*`) are included when started with `npm start`.

## Load test
`npm run build && npm run loadtest` sends `POST /payments` at an open-loop 100 requests/second for 60 s (`LOADTEST_RPS`, `LOADTEST_SECONDS`)
with Hyperswitch replaced by an in-process stub, so it measures cost-router's own path (decision, forward, attempt read-back, stats, trace).
Each run prints p50/p90/p95/p99, throughput, errors and duration, and saves them to `loadtest-results/<timestamp>.json` (git-ignored).
`npm run loadtest -- --update-docs` also rewrites the committed `docs/LOADTEST.md`. Last recorded run: 6000 requests, 100 RPS, 0 errors,
p50 3.4 / p95 5.8 / p99 9.2 ms (target p99 < 200 ms).

## State file (optional JSON "database")
Set `state_file` (the example uses `data/state.json`, git-ignored) or `COST_ROUTER_STATE_FILE` to keep statistics, attempt
de-duplication and stored decisions in **one** human-readable JSON file, so they survive a restart together (saving only the counters
would let a restart re-count attempts and forget idempotent replays). Unset, everything stays in memory as before.
- **Safe writes:** the whole file is written to a temp file and renamed over the old one, so a crash mid-write keeps the previous file.
- **One process only:** startup takes `<file>.lock` holding its pid. A second live cost-router on the same file refuses to start; a lock left
  by a crashed or killed process is taken over with a warning.
- **Invalid file:** startup stops with a clear error (fix or delete the file). A failed write is logged and never fails a payment.
- **Speed:** the file is rewritten after each decision and each completed payment. Fine for a demo or low volume, not for high throughput.
- For tests you can pre-fill it (for example a connector with past failures) instead of sending payments first. The demo does not use it:
  each scenario starts from empty stats on purpose.

## Scoring (read this before trusting it)
`score = s_hat * fee + (1 - s_hat) * L_eff`, `L_eff = failure_penalty_pct/100 * amount` (clamped to >= the highest candidate fee), lowest wins,
ties by higher `s_hat`, lower fee, connector name, merchant connector id. Money is integer minor units (BigInt, round half up); scores and rates are compared as exact fractions.

**This is a single-connector approximation. It is not the optimal fallback-chain calculation when Hyperswitch retries are enabled**: the same
order is used as the fallback order, but later connectors are not priced into the first one's score.
Example (A: fee 5000, 96%; B: fee 2000, 85%): B wins when `L_eff` is below about 28182, A above.

## Hyperswitch behaviour we rely on (verified live, see `docs/INTEGRATION_RESULTS.md`)
- `POST /payments` accepts the priority `routing` list and the **first attempt** follows its order.
- The create response has **no `attempts`**; `GET /payments/{id}?expand_attempts=true` does, which is what cost-router reads.
- The override is a **preference, not a guarantee**: a disabled, ineligible or unknown connector is silently replaced; with profile auto-retry on,
  retries can continue to connectors **outside** the list; `single` is not "no failover". A connector excluded by our floor can therefore still run on a retry.
- Auto-retry needs the profile setting **and** a GSM rule with `decision: retry` (`sub_flow` exactly `Authorize`).

## Layout
- `src/engine/`: pure, framework-free decision engine (no Nest/HTTP/storage, no clock, no randomness; portable to a Rust `RoutingStage`).
  Pipeline: `evaluate -> CandidateFilter[] -> Scorer -> rank`. A new rule is a new filter or scorer.
- `src/stats/`: `AttemptStatsStore` interface, in-memory rolling window, bounded attempt de-duplication.
- `src/hyperswitch/`: `HyperswitchClient` and the single place the `routing` body is built.
- `src/payments/`: `POST /payments`, `GET /payments/:id/routing-trace`, `DecisionStore` (bounded, in memory), request fingerprint, execution record.
- `test/`: engine, config, stats, de-duplication, decision store and adapter tests (the adapter tests run against a local fake Hyperswitch).
- `scripts/`: `up.sh`, `down.sh`, `demo.sh`/`demo.mjs`, `loadtest.mjs`, `run-bash.mjs` (finds Git Bash on Windows for the npm scripts), `up.ps1`/`demo.ps1`/`down.ps1` (PowerShell wrappers), `hyperswitch-probe.sh` (optional, needs python). `docker/`: compose override, one patched `docker_compose.toml` for the prebuilt image, seed script copy (LF).

## Limits
Stats, decisions and idempotency records are per process: in memory, or in one JSON state file when `state_file` is set (then they survive a
restart). Instances never share them, pending attempts
are never re-resolved (no polling or webhooks). Timeouts and ambiguous connector responses were **not tested**. Full list: `docs/KNOWN_LIMITATIONS.md`.
