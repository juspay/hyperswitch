# Payment Method Modular Service — Metrics, Latency & Alerting Plan

Scope: `payment_methods` and `payment_method_session` endpoints (routes/handlers in
`crates/router/src/routes/payment_methods.rs`, core logic in
`crates/router/src/core/payment_methods.rs`, `core/payment_methods/{cards,vault,network_tokenization}.rs`).

This plan is modeled on two reference points:

1. **What Payments already does in this repo** — hand-rolled business counters in
   `crates/router/src/routes/metrics.rs` incremented at success/failure points in `core/payments*`
   (`PAYMENT_COUNT`, `SUCCESSFUL_PAYMENT`, `MANDATE_COUNT`, etc.), plus generic per-request
   latency from the Actix middleware and `server_wrap`.
2. **hyperswitch-card-vault PR [#184](https://github.com/juspay/hyperswitch-card-vault/pull/184)
   and [#190](https://github.com/juspay/hyperswitch-card-vault/pull/190)** — a full observability
   pass on the locker service: metrics for secret-manager calls, in-flight requests, middleware
   operations, rate limiting, health checks, DB query/pool state, external HTTP calls, cache
   hit/miss, TTL cleanups, and domain-level outcome counters. #190 later swapped the hand-rolled
   OTel setup for a shared `metrics_utils` crate — not relevant here since router already has its
   own macro set (`counter_metric!`, `histogram_metric_f64!`, `gauge_metric!`,
   `metric_attributes!` in `crates/router_env/src/metrics.rs`), just missing
   `up_down_counter_metric!` for in-flight gauges.

The card-vault PR is the right template because payment-methods, like the locker, is fundamentally
a **store/fetch/delete-on-a-vault** service, not a payment-authorization flow — its risk surface is
locker latency, tokenization failures, and DB/cache health, not connector routing.

---

## 1. What already exists (don't re-build)

| Coverage | Mechanism | Source | v1 / v2 |
|---|---|---|---|
| Request count + latency by path/method/status | `RequestResponseMetrics` Actix middleware → `REQUESTS_RECEIVED`, `REQUEST_TIME` | `crates/router/src/middleware.rs:334-415`, registered globally in `crates/router/src/lib.rs:550` | both |
| Request count + latency by flow name, error/status in analytics pipeline | `api::server_wrap` → `ApiEvent` (Kafka/Clickhouse) | `crates/router/src/services/api.rs:397` | both |
| Locker add/delete latency + failure count | `CARD_ADD_TIME`/`CARD_DELETE_TIME`, `CARD_LOCKER_FAILURES` | `crates/router/src/core/payment_methods/cards.rs:984-1030` | **v1 only** — `add_card_to_locker` is `#[cfg(feature = "v1")]` ([cards.rs:970](crates/router/src/core/payment_methods/cards.rs#L970)) |
| Locker get latency | `CARD_GET_TIME` | `crates/router/src/core/payment_methods/cards.rs:3013-3036` | v1 |
| Temp-locker (vault) tokenize/detokenize/delete latency + failures | `CREATED_TOKENIZED_CARD`, `GET_TOKENIZED_CARD`, `DELETED_TOKENIZED_CARD`, `TEMP_LOCKER_FAILURES` | `crates/router/src/core/payment_methods/vault.rs:1689-1955` | both (temp-locker, not the v2 vault service) |
| Network tokenization latency (generate/fetch/status/delete) | `GENERATE_NETWORK_TOKEN_TIME`, `FETCH_NETWORK_TOKEN_TIME`, etc. | `crates/router/src/core/payment_methods/network_tokenization.rs:433-1308` | both |
| Background retry/cleanup counters | `RETRIED_DELETE_DATA_COUNT`, `TASKS_RESET_COUNT` | `crates/router/src/core/payment_methods/vault.rs:3053-3096` | both |
| **v2 vault service calls (add/get-fingerprint/retrieve/delete)** | **none** | `vault::call_to_vault::<V: VaultingInterface>` — `crates/router/src/core/payment_methods/vault.rs:1990-2022`, called from `cards.rs:790,844,898,951,1407,1439,1474,6547,6590` | **v2 only — this is the modular payment method service's actual vault call, and it has zero latency metrics today** |
| **DB query count + latency (all tables, all operations)** | `DATABASE_CALLS_COUNT`, `DATABASE_CALL_TIME` — tagged by `table`, `operation` | `track_database_call` wrapper inside `generic_find_one`/`generic_insert`/`generic_update`/etc. — `crates/diesel_models/src/query/generics.rs:45-64`; metrics defined `crates/diesel_models/src/lib.rs:265-266` | both — applies to every diesel-backed entity in the app, including `PaymentMethod` (verified via `crates/diesel_models/src/query/payment_method.rs:22,61,72,233`, all of which call the generic functions above) |

**Correction:** an earlier pass of this doc claimed no DB query instrumentation exists anywhere in
router. That was wrong — it exists, and it's actually the strongest-leveraged metric in the whole
stack: because every diesel store implementation is required to go through the shared
`generic_find_one`/`generic_insert`/`generic_update` functions in `query/generics.rs`, one wrapper
at that single layer already gives count + latency, per table + operation, for every entity in the
app — no per-call-site work needed, unlike card-vault's PR #184 which had to hand-wrap every query
individually (see §3.2 note) because their storage layer builds each query inline with no shared
generic executor. **Nothing needs to be added for DB latency — it's already there.**

**Gap #1 — the important one for this launch:** the payment-method-modular-service (v2) vault call
path has *no* latency metric at all. `CARD_ADD_TIME`/`CARD_GET_TIME`/`CARD_DELETE_TIME` look like
they'd cover it but don't — they're compiled only under `feature = "v1"` and instrument a
completely different function (`add_card_to_locker`/`delete_card_from_locker`, which call the
legacy card-vault HTTP client directly). The v2 flow instead calls the generic
`vault::call_to_vault<V>` at `vault.rs:1990`, and nothing wraps it. See §2.3 for the fix — this
needs new `VAULT_*` metrics, not a reuse of the `CARD_*` ones.

**Gap #2:** there is no business-outcome counter anywhere in `core/payment_methods.rs` (the
7.9k-line module with `create_payment_method_core`, `list_customer_payment_methods_core`,
`delete_payment_method_core`, etc.) or in the seven `payment_method_session_*` route handlers.
Section 2 fills that gap; section 3 covers what the card-vault PR added that has no analogue here
at all (active-request gauge, cache hit/miss, external HTTP call metrics — DB is no longer in this
list, see the correction above).

---

## 2. New metrics to add — business/flow layer (mirrors `PAYMENT_COUNT`/`SUCCESSFUL_PAYMENT`)

Add to `crates/router/src/routes/metrics.rs`, next to the existing payment counters. All use
`metric_attributes!` for a `flow`/`outcome` label so a single counter covers success and failure
(matches the card-vault `outcome="success"|"error"` label convention).

### 2.1 `payment_methods` (create / list / retrieve / update / delete)

```rust
counter_metric!(
    PAYMENT_METHOD_OPS_COUNT, GLOBAL_METER,
    "Number of payment_methods operation attempts, tagged by operation and outcome"
);
histogram_metric_f64!(
    PAYMENT_METHOD_OPERATION_DURATION, GLOBAL_METER,
    "Duration of payment_methods core operations"
);
```

Labels: `operation` = `create` | `create_for_intent` | `create_for_confirm` | `list` | `retrieve` |
`update` | `delete`; `outcome` = `success` | `error`.

Call sites (increment at the end of each core function, both success and error branches):

- `create_payment_method_core` — `crates/router/src/core/payment_methods.rs:1648`
- `create_payment_method_for_intent` / `create_payment_method_for_confirm` — `:4438` / `:4595`
- `list_customer_payment_methods_core` — `:5612`
- `delete_payment_method_core` — `:6442`

This is the direct analogue of `PAYMENT_COUNT`/`SUCCESSFUL_PAYMENT` — without it, a spike in
payment-method create failures (e.g. locker down) is invisible except as generic 5xx rate on the
path, with no way to distinguish "vault save failed" from "bad request."

### 2.2 `payment_method_session` (the modular-service-specific flows)

```rust
counter_metric!(
    PAYMENT_METHOD_SESSION_OPS_COUNT, GLOBAL_METER,
    "Number of payment_method_session operation attempts, tagged by operation and outcome"
);
histogram_metric_f64!(
    PAYMENT_METHOD_SESSION_OPERATION_DURATION, GLOBAL_METER,
    "Duration of payment_method_session operations"
);
counter_metric!(
    SUCCESSFUL_PAYMENT_METHOD_SESSION_CONFIRM, GLOBAL_METER,
    "Number of successful payment_method_session confirmations"
);
```

Labels: `operation` = `session_create` | `session_update` | `session_retrieve` |
`session_list_payment_methods` | `session_confirm` | `session_update_saved_pm` |
`session_delete_saved_pm`.

Call sites — the seven handlers in `crates/router/src/routes/payment_methods.rs:1668-1962`
(`payment_methods_session_create`, `_update`, `_retrieve`,
`payment_method_session_list_payment_methods`, `_confirm`,
`_update_saved_payment_method`, `_delete_saved_payment_method`). `session_confirm` is the
highest-value one to break out separately (mirrors `SUCCESSFUL_PAYMENT`) since it's the flow that
actually completes a save-and-use-for-payment action — a silent regression there is the closest
equivalent to a failed payment.

### 2.3 v2 vault call latency — `VAULT_ADD_TIME` / `VAULT_GET_TIME` / `VAULT_DELETE_TIME`

This is the v2 analogue of `CARD_ADD_TIME`/`CARD_GET_TIME`/`CARD_DELETE_TIME`, and it's the one
piece of "obvious infra coverage" that's actually missing rather than already present (see Gap #1
in §1) — the v1 metrics don't extend to v2 because they instrument a different function entirely.

```rust
histogram_metric_f64!(VAULT_ADD_TIME, GLOBAL_METER);
histogram_metric_f64!(VAULT_GET_TIME, GLOBAL_METER);
histogram_metric_f64!(VAULT_DELETE_TIME, GLOBAL_METER);
histogram_metric_f64!(VAULT_FINGERPRINT_TIME, GLOBAL_METER);
counter_metric!(VAULT_CALL_FAILURES, GLOBAL_METER);
```

**Instrument once at the choke point, not at each call site.** `vault::call_to_vault<V:
VaultingInterface>` (`crates/router/src/core/payment_methods/vault.rs:1990-2022`) is the single
generic function all v2 vault operations go through — `AddVault`, `GetVaultFingerprint`,
`VaultRetrieve`, `VaultDelete` (trait impls in `crates/router/src/types/payment_methods.rs:130-168`)
each already expose a clean name via `V::get_vaulting_flow_name()`. Wrapping `call_to_vault` itself
means all 9 current call sites (`cards.rs:790, 844, 898, 951, 1407, 1439, 1474, 6547, 6590`) get
covered automatically, and any future call site does too — no per-call-site edits needed, unlike
the v1 pattern which instruments each of `add_card_to_locker`/`delete_card_from_locker` separately.

Dispatch to the right histogram by matching on `V::get_vaulting_flow_name()` (or add a small
`fn metric_histogram() -> &'static Histogram<f64>` default method to `VaultingInterface` if that
reads cleaner), wrap the `services::call_connector_api` call at `vault.rs:2020` with
`record_operation_time`, and increment `VAULT_CALL_FAILURES` with an `operation` attribute on the
error branch — same shape as `CARD_LOCKER_FAILURES` in `cards.rs:990`.

This — combined with §3.1's encryption metric — is what actually completes the "split total
latency by dependency" answer for the v2/modular flow specifically: `PAYMENT_METHOD_OPERATION_DURATION`
(total) − `VAULT_ADD_TIME`/`VAULT_GET_TIME` (vault) − `PAYMENT_METHOD_ENCRYPTION_DURATION`
(keymanager) = remainder attributable to DB/business logic.

### 2.4 Outcome enum (optional, cleaner than string labels)

Card-vault uses a `strum::IntoStaticStr` enum for outcome labels instead of raw strings
(`DomainGetOrInsertOutcome`, `TtlDeletionOutcome` in their `metrics.rs`). Router doesn't currently
follow that pattern for payments metrics (uses string literals directly in
`metric_attributes!`), so for consistency with the existing codebase style, stick with string
literals unless this is expanded further — don't introduce a new convention for one module.

---

## 3. New metrics to add — infra layer (things card-vault has that router doesn't, for this path)

### 3.1 Encryption/keymanager latency — required, not optional

This is the one real gap in the "split total latency by downstream dependency" story. Locker,
temp-locker, and network-tokenization calls already have per-call latency histograms (§1), but
every `encrypt_data`/`decrypt` call in the payment-methods path has **zero** timing today:

- `encrypt_data` calls — `crates/router/src/core/payment_methods.rs:6680`, `:6807`
- `DecryptOptional` calls — `crates/router/src/core/payment_methods/cards.rs:3068`, `:6201` (the
  same read path that already records `CARD_GET_TIME`/`GET_FROM_LOCKER`, so today that number
  silently includes an un-attributed decrypt cost)

Without this, `total (PAYMENT_METHOD_OPERATION_DURATION) − locker slice (CARD_*_TIME)` lumps
encryption-service time, DB time, and business logic into one unexplained remainder — you can't
tell whether a latency regression is the locker or the keymanager. Add:

```rust
histogram_metric_f64!(
    PAYMENT_METHOD_ENCRYPTION_DURATION, GLOBAL_METER,
    "Duration of encrypt/decrypt calls to the keymanager during payment_methods operations"
);
```

Labels: `operation` = `encrypt` | `decrypt`; `outcome` = `success` | `error`. Wrap each of the four
call sites above with `record_operation_time` (same helper already used for `CARD_ADD_TIME` etc.
in `cards.rs`).

### 3.2 Everything else card-vault has that's genuinely out of scope for this launch

| Metric | Type | Why it matters here specifically |
|---|---|---|
| `PAYMENT_METHOD_ACTIVE_REQUESTS` | up-down counter (gauge via Prometheus) | In-flight `payment_methods`/`payment_method_session` requests. Needs `up_down_counter_metric!` added to `crates/router_env/src/metrics.rs` (missing today — card-vault added exactly this in PR #184). Lets you alert on saturation independent of latency (e.g. locker slow but not yet timing out). |
| `PAYMENT_METHOD_REDIS_CACHE_LOOKUP_COUNT` | counter, labels `cache`, `outcome=hit\|miss` | If payment-methods list/retrieve reads through a Redis cache (customer PM list caching), instrument hit/miss the way card-vault does `CACHE_LOOKUP_COUNT`. Confirm cache usage in `list_customer_payment_methods_core` before adding — don't add unused metrics. |
| `PAYMENT_METHOD_EXTERNAL_CALL_DURATION` | histogram, labels `target` (`locker`\|`network_tokenization_service`\|`keymanager`), `outcome` | Generalizes the existing per-target latency metrics (`CARD_ADD_TIME`, `GENERATE_NETWORK_TOKEN_TIME`, §3.1's encryption metric) under one queryable metric family, matching card-vault's single `EXTERNAL_HTTP_REQUEST_DURATION` instead of one histogram per call site. Optional consolidation — the per-target histograms already give equivalent coverage individually; only worth doing if a unified "external dependency health" dashboard panel is wanted. |

(DB query timing was previously listed here as a missing, cross-cutting item — that was wrong; see
the correction in §1. It already exists via `DATABASE_CALLS_COUNT`/`DATABASE_CALL_TIME` and needs
no new code.) Cache hit/miss is the remaining cross-cutting unknown — flag to the team as a
follow-up pending confirmation of whether payment-methods reads through a cache at all. The
active-request gauge is cheap (one macro port + wrap) and worth including in this launch if time
allows, but isn't required to get a complete latency breakdown the way §2.3/§3.1 are.

### 3.3 What "complete breakdown" looks like once §2.3 + §3.1 land

For a v2 `create_payment_method` call: `PAYMENT_METHOD_OPERATION_DURATION{operation="create"}`
gives the total. Subtract:
- `VAULT_ADD_TIME` (§2.3, the actual vault call this flow makes)
- `PAYMENT_METHOD_ENCRYPTION_DURATION{operation="encrypt"}` (§3.1)
- `DATABASE_CALL_TIME{table="payment_method",operation="Insert"}` (already exists, §1)

and what's left is router-side business logic only — essentially the full latency waterfall for
this endpoint, not an "unexplained remainder." Without §2.3 and §3.1, the remainder would silently
include vault and encryption time, which (being the two network calls in that remainder, unlike
business logic or the now-visible DB call) are the pieces most likely to actually be the regression
source.

---

## 4. Alerting rules to define (once metrics exist)

`config/prometheus.yaml` currently has `rule_files:`/`alerting:` commented out — **no alert rules
exist in this repo for any flow**, payments included. These need to be written from scratch,
either in this repo's Prometheus config or in whatever external Grafana/Prometheus actually serves
prod (need to confirm — see open question below). Proposed rules, once section 2 metrics land:

| Alert | Condition | Severity |
|---|---|---|
| PaymentMethodCreateErrorRateHigh | `rate(PAYMENT_METHOD_OPS_COUNT{operation="create",outcome="error"}[5m]) / rate(PAYMENT_METHOD_OPS_COUNT{operation="create"}[5m]) > 0.05` for 5m | page |
| PaymentMethodSessionConfirmErrorRateHigh | same shape on `session_confirm` | page |
| PaymentMethodLatencyP99High | `histogram_quantile(0.99, PAYMENT_METHOD_OPERATION_DURATION) > <SLO, e.g. 2s>` for 10m | warn |
| VaultCallFailureRateHigh | `rate(VAULT_CALL_FAILURES[5m]) > 0` sustained for 5m — the v2/modular equivalent; `LockerCallFailureRateHigh` on `CARD_LOCKER_FAILURES` is v1-only and won't fire for the modular service | page |
| TempLockerFailureRateHigh | `rate(TEMP_LOCKER_FAILURES[5m]) > 0` sustained for 5m | page |
| PaymentMethodActiveRequestsSaturated | `PAYMENT_METHOD_ACTIVE_REQUESTS > <capacity threshold>` | warn |
| GenericPaymentMethods5xxRate | `rate(REQUEST_TIME{path=~"/payment_methods.*",status_code=~"5.."}[5m])` high — usable **today**, no new metric needed | page |

The last row can be wired immediately since `REQUEST_TIME`/`REQUESTS_RECEIVED` already carry
`path`/`status_code` labels — it doesn't need to wait on section 2.

---

## 5. Rollout checklist

- [ ] Add `up_down_counter_metric!` macro to `crates/router_env/src/metrics.rs` (port from
      card-vault PR #184's `observability/macros.rs`)
- [ ] Add business counters/histograms from §2.1/§2.2 to `crates/router/src/routes/metrics.rs`
- [ ] Instrument the 4 core functions (§2.1) and 7 route handlers (§2.2) with success/error
      increments
- [ ] Add `VAULT_ADD_TIME`/`VAULT_GET_TIME`/`VAULT_DELETE_TIME`/`VAULT_FINGERPRINT_TIME`/
      `VAULT_CALL_FAILURES` (§2.3) and wrap `vault::call_to_vault` once at
      `vault.rs:1990-2022` — this is the actual v2 vault latency metric; do **not** assume
      `CARD_ADD_TIME` etc. cover it, they're `#[cfg(feature = "v1")]` only
- [ ] Add `PAYMENT_METHOD_ENCRYPTION_DURATION` (§3.1) and wrap the 4 encrypt/decrypt call sites —
      required for a complete latency split, not optional
- [ ] Confirm whether prod dashboards/alerts live in this repo's `config/prometheus.yaml` or an
      external Grafana/Prometheus instance — **open question, need input** — before writing actual
      alert-rule YAML/dashboard JSON
- [ ] Wire the "usable today" generic 5xx-rate alert (§4 last row) independent of the above, since
      it needs no code change
- [ ] Wire a `DATABASE_CALL_TIME{table=~"payment_method.*"}` p99 alert — usable **today**, no code
      change needed, since this metric already exists (§1 correction)
- [ ] Decide whether cache hit/miss (§3.2) is in scope for this launch or tracked as a separate
      follow-up — first confirm payment-methods actually reads through a cache
      instrumentation exists anywhere in router today

---

## Open question

Section 4's alert rules assume a destination (Prometheus rule files, Grafana alerting, Datadog,
etc.). This repo has no dashboards/alert rules checked in for *any* flow, including payments — so
there's nothing to copy from. Need to know where the real payments dashboards/alerts actually live
(internal Grafana instance? separate IaC repo?) to draft the payment-methods equivalents against
the same system rather than inventing thresholds blind.
