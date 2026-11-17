# Load test: 100 RPS for 60 s on the custom path

Command: `npm run build && npm run loadtest` (`scripts/loadtest.mjs`). Run on 2026-11-17, Windows 10, 8 GB RAM, Node v24.21.0.

**What is measured:** `POST /payments` through cost-router at an open-loop 100 requests/second (each request is sent on schedule,
whether or not earlier ones finished), each with a new Idempotency-Key. Every request runs the full custom path: routing decision,
forward to Hyperswitch, attempt read-back (`GET ...?expand_attempts=true`), stats update, trace storage.

**What is not measured:** Hyperswitch and the connectors. They are replaced by an in-process stub that answers immediately, so this is
cost-router's own latency. Against the real stack, Hyperswitch's and the PSP's time is added on top.

| Metric | Value |
|---|---|
| Requests | 6000 (100 RPS achieved) |
| Errors | 0 ({"200":6000}) |
| p50 / p90 / **p99** / max | 3.2 / 4.7 / **10.3** / 85.7 ms |
| Decisions counted on `/metrics` | 6000 |
| Result | **PASS** (target p99 < 200 ms) |
