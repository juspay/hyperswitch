# DECISIONS

**Built:** cost-aware routing as a NestJS layer (`cost-router/`) in front of an unmodified Hyperswitch. Per payment it ranks connectors by
`score = s_hat·fee + (1 − s_hat)·L_eff` (fee charged on success; `L_eff = failure_penalty_pct × amount`, at least the highest fee), sends the
order as `routing: {type: priority}`, reads per-attempt outcomes back, and keeps a trace (`GET /v1/payments/{id}/routing-trace`). A cheap flaky
PSP loses to a dearer reliable one once its failures show; below a reliability floor the layer steps aside and Hyperswitch routes normally.

**Why a layer, not Rust in the router:** this machine (8 GB, no Rust toolchain) could not realistically compile the router. The engine is a
pure module (no framework, clock or I/O), so porting it into a `RoutingStage` is mechanical. Cost: less in-Hyperswitch depth, and I did not
confirm this reading of the brief with the founders. No Hyperswitch file was changed.

**Decisions:** integer minor units end to end. `failure_penalty_pct` and `min_success_rate` are required with **no default** (a default would be
an invented business number). Reliability is measured per **attempt**, since after a retry the final status credits the wrong connector. One
pooled prior for shrinkage. Unpriced connectors rank last; no usable pricing means no override. Production instinct: idempotent by payment id
(same key, different body gives 409), only a request hash stored, no card data in traces.

**Bonus:** trace endpoint; Prometheus `GET /metrics` per connector; load test 100 RPS × 60 s, p99 9.2 ms with a stubbed Hyperswitch (`docs/LOADTEST.md`).

**Skipped:** circuit breaker, Redis/Postgres (an optional JSON state file instead), latency routing, BIN-level stats, Rust port.

**Broken or hacky:**
- The score is a **single-connector approximation**, not the optimal fallback-chain cost when retries are on.
- Hyperswitch treats the override as a **preference**: unknown or disabled connectors are silently replaced, and with auto-retry on, retries go
  past our list, so a connector the floor excluded can still run. The trace keeps requested and actual execution apart.
- Pending attempts are never re-resolved (no webhooks); issuer declines count against a connector by default; a few early failures with no
  successes put every connector under the floor.
- State is per process (memory or one JSON file), not shared between instances. Timeouts and ambiguous responses are **not tested**.
- The prebuilt image needed a trimmed config copy and Superposition; `make demo` is untested here (no `make`), `npm run demo` is what ran.
- Caught by running it: my first prior (peers only) ranked a connector with one success below one with one failure (fixed, regression test);
  the assumption that `single` routing means "no failover" was wrong.

**Time:** about 2 hours of hands-on work. WSL2/Docker installation (admin steps, reboot) and image downloads took several more hours on this
machine and are not counted; I worked on other things in between.

**With 4 more hours:** chain-aware scoring from a measured recovery rate and a way to keep retries inside the list; shared durable stats with
webhook ingestion for pending attempts; timeout and double-charge tests, and the Rust `RoutingStage` port.
