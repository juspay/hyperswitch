# Known limitations (DECISIONS-ready)

Accepted for this submission:

1. **Stats are per process.** Multiple cost-router instances do not share stats, so their rankings can diverge. With the optional
   `state_file`, only one process may use a file (lock); it is rewritten on every decision and payment, so it is not for high throughput.
2. **Without `state_file`, a restart loses all stats** (and the trace store and de-duplication memory). With it, all three are
   restored together from one JSON file. A lock whose pid was reused by an unrelated process blocks startup until the lock file is deleted. Nothing survives a process restart; after one, every
   connector starts with no data and shrinks toward the prior.
3. **Pending attempts are never re-resolved.** There is no polling and no webhook ingestion, and attempts are read only once, from
   the create response. An attempt first seen as pending stays "unresolved" and never contributes a success or a failure, so
   connectors with many pending attempts (for example 3DS flows) feed less data into their success rate. The lifecycle logic itself
   is ready for later reads: a pending sighting counts once as unresolved, the first terminal sighting counts once as its terminal
   class, duplicates are ignored, and a pending sighting that arrives after the terminal one is ignored.
   **First terminal event wins:** if two different terminal statuses are ever reported for the same attempt (for example failure
   then charged), only the first is counted; later ones are never reclassified. This assumes terminal statuses are final.
   Key: `(paymentId, attemptId, stage)`, stage being `unresolved` or `terminal`.
4. **De-duplication is bounded.** An attempt older than the dedupe TTL or evicted by the size cap, if seen again, is counted again.
   Attempts without `attempt_id` are not counted at all (warning logged).
5. **Score is a single-connector approximation**, not the optimal fallback-chain cost when Hyperswitch retries are enabled.
6. **Hyperswitch treats the routing override as a preference (verified live, `docs/INTEGRATION_RESULTS.md`).** A disabled, ineligible or unknown
   connector is silently replaced; entries without `merchant_connector_id` are ignored; with profile auto-retry on, retries can continue to
   connectors outside our list, so a connector excluded by the reliability floor can still run on a retry. `single` is not "no failover".
   The trace therefore records requested routing (`decision`) and actual execution (`execution`) separately and claims no attempt order.
   Still **not tested**: timeouts and ambiguous connector responses (double-charge risk), because the dummy connector cannot simulate them.
7. **Decisions are bounded and in-memory.** The decision store (idempotent replay, `routing-trace`) expires entries after `decision_ttl_minutes` and evicts oldest-first beyond `decision_max_entries` (in-flight requests are never evicted). After expiry a replayed payment id is treated as a new payment (a duplicate Hyperswitch `payment_id` is then rejected by Hyperswitch itself, `DuplicatePayment`), and the trace returns 404. Without `state_file`, replays are also lost on restart; they are never shared between instances.
8. **Issuer declines can penalise connectors** while `issuer_decline_codes` is empty (the default).
9. **Thin data and the floor.** `s_hat` is shrunk toward a common pooled prior, so with a few early failures and no successes anywhere every connector
   falls under `min_success_rate` and the layer defers to Hyperswitch until successes accumulate.
10. **The create response has no `attempts`**; cost-router makes a second call (`GET /payments/{id}?expand_attempts=true`) per payment. If it
    fails, the payment still succeeds, but no stats are recorded for it and the trace's `execution.attempts` is `null`.
11. **`make demo` is untested on the dev machine** (no `make`); `npm run demo` / `bash scripts/demo.sh` is what was run.
