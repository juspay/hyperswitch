# DECISIONS

**Built:** cost-aware routing as a NestJS layer (`cost-router/`) in front of an unmodified Hyperswitch. Per payment it ranks connectors by
`score = s_hat·fee + (1 − s_hat)·L_eff` (fee only on success; `L_eff = failure_penalty_pct × amount`, clamped to the highest fee), sends the
order as `routing: {type: priority}`, reads per-attempt outcomes back, and stores a trace (`GET /payments/{id}/routing-trace`). A cheap flaky PSP
loses to a dearer reliable one once its failures show; below a reliability floor the layer steps aside and Hyperswitch routes normally.

**Why a layer, not Rust inside the router:** this machine (8 GB, no Rust toolchain) cannot realistically compile the router. The engine is a pure
module (no framework, clock or I/O), so porting it into a `RoutingStage` is mechanical. Cost: less in-Hyperswitch depth, and I did not confirm
with the founders that this reading of the brief is acceptable. No Hyperswitch file was changed.

**Decisions:** integer minor units end to end. `failure_penalty_pct` and `min_success_rate` are required, **no default** (a default would be an
invented business number). Reliability is measured per **attempt**, not per payment: after a retry the final status would credit the wrong
connector. One common pooled prior for shrinkage. Unpriced connectors rank last; no usable pricing means no override. Production instinct:
idempotent by payment id (same key, different body gives 409), only a request hash is stored, no card data in traces.

**Skipped:** circuit breaker, Redis/Postgres (a JSON state file instead), latency, BIN/issuer-level stats, load test, Rust port. Prometheus
metrics per connector are at `GET /metrics`.

**Time:** about 2 hours of hands-on design and implementation. Installing WSL2 and Docker on this machine (admin steps, a reboot) and
pulling images took several more hours and are not counted; I was working on other things in between, so the session spans longer.

**Broken or hacky:**
- The score is a **single-connector approximation**, not the optimal fallback-chain cost when retries are on.
- Hyperswitch treats the override as a **preference** (verified): disabled or unknown connectors are silently replaced, and with profile auto-retry on,
  retries go **past our list**, so a connector the floor excluded can still run. The trace keeps requested vs actual execution apart and claims no
  attempt order (`attempts[]` order is not guaranteed).
- Pending attempts are never re-resolved (one read, no webhooks); first terminal status wins; with the default empty `issuer_decline_codes`,
  issuer declines count against a connector. A few early failures with no successes put every connector under the floor.
- Stats, idempotency and traces are per process: in memory, or in one JSON state file (temp-file-and-rename writes, single-process lock)
  that survives restarts but is rewritten on every payment, so it is not for high volume and not shared between instances.
- Timeouts and ambiguous responses are **not tested**. The prebuilt image needed a trimmed config copy and Superposition. `make demo` is untested
  (no `make` here); `npm run demo` is what ran.
- Caught by running it: my first prior (peers only) ranked a connector with one success below one with one failure (fixed, regression test), and my
  assumption that `single` routing means "no failover" was wrong.

**With 4 more hours:** (1) chain-aware scoring from a measured recovery rate, plus a way to keep retries inside the list; (2) shared durable stats
(Postgres/Redis instead of the JSON file) with webhook ingestion for pending attempts; (3) timeout and double-charge tests, a 100 RPS load test, the Rust port.
