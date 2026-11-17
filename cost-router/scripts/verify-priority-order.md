# Priority order verification: superseded

This was a how-to written before a live run. It has been done: see `docs/INTEGRATION_RESULTS.md` (Part B, questions 1-4) and run
`bash scripts/demo.sh`, which checks the order, the fallback and the trace against a real Hyperswitch.

Verified: `POST /payments` accepts the priority list and the first attempt follows its order. Also verified, and important: the override is a
preference, not a guarantee (disabled or unknown connectors are silently replaced, and retries can go outside the list when profile auto-retry is on).
Not tested: timeouts and ambiguous connector responses.
