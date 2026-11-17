# Attempt outcome classification

Source of truth in code: `src/engine/outcome.ts` (`ATTEMPT_STATUS_TABLE`). A test asserts every
`AttemptStatus` variant (29, `crates/common_enums/src/enums.rs`, wire format snake_case) is classified.
Reliability is measured **per attempt** (the `attempts` list), not per final payment outcome.

Status: owner-approved for the MVP (authorized = success; authentication_failed, router_declined and every capture/void/refund status ignored; unclassified definitive failures technical; integrity_failure its own class).

| Class | Counted in n? | Statuses |
|---|---|---|
| `success` | yes (numerator and n) | charged, authorized, partial_charged, partially_authorized, partial_charged_and_chargeable |
| `technical_failure` | yes (n only) | failure / authorization_failed whose error code is NOT in `issuer_decline_codes` |
| `issuer_decline` | no | failure / authorization_failed whose error code IS in `issuer_decline_codes` |
| `integrity_failure` | no | integrity_failure. Own class: connector responsibility is not established (an integrity mismatch between request and response data can originate on our side or the router), so it is neither a success nor counted against the connector. Revisit if evidence shows it is the connector's fault. |
| `unresolved` | no | started, pending, authorizing, authentication_pending, authentication_successful, confirmation_awaited, payment_method_awaited, device_data_collection_pending, unresolved, cod_initiated, expired |
| `ignored` | no | authentication_failed, router_declined, voided, voided_post_charge, void_initiated, capture_initiated, capture_failed, void_failed, auto_refunded, capture_review; and any status string not in the table |

`n = successes + technical_failures`. Estimate: `s_hat = (successes + k*prior) / (n + k)`, `k = min_sample_size`,
`prior` = pooled success rate of the other candidates that have data, or 1.0 when none has. A prior common to
all candidates cancels in the ranking when sample sizes are equal; it only matters through differing sample sizes.
The result also exposes the observed count per class.

## Decisions and caveats
1. **Unclassified definitive failure = technical** (MVP). `issuer_decline_codes` defaults to empty, so until it is filled in
   every `failure` / `authorization_failed` counts against its connector. A connector that happens to receive more issuer
   declines (insufficient funds, do-not-honour) is then penalised unfairly. Fill the list from observed error codes.
2. **`integrity_failure` is excluded from reliability** (own class) because connector responsibility is not established.
3. **Unknown statuses are `ignored` and produce a warning** (returned by `classifyAttempt`, logged by the adapter), never silent.
4. **Pending/unresolved statuses are never counted as outcomes** and are never re-resolved (no polling/webhooks in this submission).
5. Whether real error codes separate issuer declines from technical errors in this deployment is unverified [?].

## Attempt de-duplication
Key = `(paymentId, attemptId)` (attempt ids are payment-scoped, so the pair is the safe choice). The key is claimed synchronously
before anything can yield, so interleaved ingestions of the same attempt count once. Memory is bounded by a TTL (>= `window_minutes`,
injected clock) and a max-size cap with oldest-first eviction; an expired or evicted attempt seen again would be counted again.
An attempt with no `attempt_id` cannot be de-duplicated: it is **not counted** and a warning is logged.
Unresolved sightings use a separate key and never consume the terminal slot: an attempt first seen `pending` and later terminal is
counted once as `unresolved` and once as its terminal class (the first terminal sighting wins; later changes are not re-classified).

## Known limits
In-memory, per process, lost on restart. See `KNOWN_LIMITATIONS.md`.
