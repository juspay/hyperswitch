# Load test: 100 RPS for 60 s on the custom path

Command: `npm run build && npm run loadtest` (`scripts/loadtest.mjs`; this file is rewritten only with `-- --update-docs`).
Run on 2026-11-17, Windows 10, 8 GB RAM, Node v24.21.0.

**What is measured:** `POST /payments` through cost-router at an open-loop 100 requests/second (each request is sent on schedule,
whether or not earlier ones finished), each with a new Idempotency-Key. Every request runs the full custom path: routing decision,
forward to Hyperswitch, attempt read-back (`GET ...?expand_attempts=true`), stats update, trace storage.

**What is not measured:** Hyperswitch and the connectors. They are replaced by an in-process stub that answers immediately, so this is
cost-router's own latency. Against the real stack, Hyperswitch's and the PSP's time is added on top.

| Metric | Value |
|---|---|
| Requests | 6000 |
| Duration | 60 s sending, 60 s until the last response |
| Throughput | 100 RPS offered, 100 RPS completed |
| Errors | 0 ({"200":6000}) |
| Latency p50 / p95 / **p99** / max | 3.4 / 5.8 / **9.2** / 91.3 ms (p90 4.9 ms) |
| Decisions counted on `/metrics` | 6000 |
| Result | **PASS** (target p99 < 200 ms) |
