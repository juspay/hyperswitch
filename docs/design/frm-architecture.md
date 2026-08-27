# FRM (Fraud & Risk Management) — Target Architecture

Status: proposal
Scope: `crates/router/src/core/fraud_check*`, `crates/hyperswitch_domain_models`, `crates/hyperswitch_interfaces`, `crates/hyperswitch_connectors`, `crates/diesel_models`, `crates/common_types`

## 1. Goal

Make FRM a **generic risk substrate** rather than a Signifyd-shaped integration, so that:

- a merchant can express *their* risk policy without us shipping code for it,
- a new FRM provider can be integrated by implementing two traits, not five,
- FRM works identically in v1 and v2,
- FRM failure modes (timeout, provider down, ambiguous verdict) are a merchant choice, not an accident of which internal call errored.

This document maps what exists, states the specific defects with file references, and proposes a layered target with a phased, back-compatible migration.

## 2. What exists today

FRM is already substantially built. Nothing here is greenfield.

| Concern | Location |
| --- | --- |
| Orchestration | `crates/router/src/core/fraud_check.rs` (885 lines) |
| Pre/Post operations | `crates/router/src/core/fraud_check/operation/{fraud_check_pre,fraud_check_post}.rs` |
| Connector flows | `crates/router/src/core/fraud_check/flows/{checkout,sale,transaction,fulfillment_flow,record_return}.rs` |
| Connector traits (v1) | `crates/hyperswitch_interfaces/src/api/fraud_check.rs` |
| Connector traits (v2) | `crates/hyperswitch_interfaces/src/api/fraud_check_v2.rs` |
| Request/response types | `crates/hyperswitch_domain_models/src/router_{request,response}_types/fraud_check.rs` |
| Providers | `signifyd`, `riskified`, `cybersourcedecisionmanager` in `crates/hyperswitch_connectors/src/connectors/` |
| Persistence | `crates/diesel_models/src/fraud_check.rs`, table `fraud_check` (`schema.rs:684`) |
| Payments integration | `crates/router/src/core/payments.rs:864-915` (pre) and `:1456-1474` (post) |
| Webhooks | `crates/router/src/core/webhooks/incoming.rs:2221` (`FrmApproved` / `FrmRejected`) |
| HTTP surface | `POST /frm_fulfillment` only (`routes/app.rs:2197-2201`) |
| Analytics | `crates/analytics/src/frm/` — `frm_triggered_attempts`, `frm_blocked_rate` |
| Gating | cargo feature `frm` + `state.conf.frm.enabled` (`config/development.toml:1401`) |

Current control flow:

```
payments_operation_core
  └─ call_frm_before_connector_call            fraud_check.rs:656
       ├─ should_call_frm                      fraud_check.rs:167   (reads merchant_account.frm_routing_algorithm)
       ├─ make_frm_data_and_fraud_check_operation                    (picks FraudCheckPre | FraudCheckPost)
       └─ pre_payment_frm_core                 fraud_check.rs:489
            └─ [Pre]  Checkout → Transaction        (may set should_continue_transaction=false)
            └─ [Post] force capture_method=Manual   (should_continue_capture=false)
  ── connector authorization ──
  └─ post_payment_frm_core                     fraud_check.rs:574
       └─ [Post] Sale → execute_post_tasks → payments_core::<Void|Capture>
```

## 3. Defects

Each is a concrete blocker to "easy, generic integration".

**D1 — The domain vocabulary is one vendor's API.**
`Checkout`, `Sale`, `Transaction`, `Fulfillment`, `RecordReturn` are Signifyd's verbs. `FraudCheck` (`api/fraud_check.rs:44`) requires all five. A provider exposing a single `POST /decision` must still implement five traits and five transformers, three of them stubs. `FraudCheckResponseData` (`router_response_types/fraud_check.rs:7`) is `#[serde(untagged)]` over three vendor-shaped variants, and `fraud_check_post.rs:424-563` is 140 lines of cross-product match arms whose only job is to detect "wrong variant for this flow" — a shape the type system should have made unrepresentable.

**D2 — Eligibility is a hand-rolled filter that cannot express what merchants ask for.**
`should_call_frm` (`fraud_check.rs:263-351`) is ~90 lines of nested `filter`/`map` over `frm_configs`, matching stringly on connector and payment method, then taking `.first()` with a panic-safety comment. It can express *gateway × payment_method → pre|post* and nothing else. It cannot express "amount over $500", "shipping country ≠ billing country", "first transaction for this customer", "BIN in list". Meanwhile the repo already has a rule engine doing exactly this for 3DS: `ThreeDSDecisionRule` + `EuclidDirFilter::ALLOWED` (`common_types/src/three_ds_decision_rule_engine.rs:66`), executed at `core/three_ds_decision_rule.rs:41-80`.

**D3 — Configuration is merchant-scoped and single-provider.**
`should_call_frm` reads `merchant_account.frm_routing_algorithm` (`fraud_check.rs:185-190`) — one FRM provider per *merchant*, shared across every profile. `business_profile` already carries `frm_routing_algorithm` (v1, `business_profile.rs:39`) and `frm_routing_algorithm_id` (v2, `:626`), both unused by this path. There is no way to run two providers, or a different provider per profile.

**D4 — Pre/Post is a hardcoded binary over a linear step machine.**
`fraud_check_operation_by_frm_preferred_flow_type` (`fraud_check.rs:475`) returns one of two boxed operations. `FraudCheckLastStep` (`diesel_models/src/enums.rs:172`) is a 4-variant *linear* enum — `Processing → CheckoutOrSale → TransactionOrRecordRefund → Fulfillment`. There is no representation for: two providers, an in-house rule pass before the vendor, a check at capture time, or a shadow evaluation.

**D5 — FRM reaches back into payments core (layering inversion).**
`execute_post_tasks` calls `payments::payments_core::<Void, …>` (`fraud_check_post.rs:264`) and `payments_core::<Capture, …>` (`:321`) directly. The action set is therefore fixed at {cancel, manual review, capture} and hardcoded in the operation. Adding "auto-refund" or "step up to 3DS" means editing FRM internals rather than declaring an action.

**D6 — v2 is unimplemented.**
14 `todo!()` in `core/fraud_check*` (including `call_frm_service` at `:70`, `should_call_frm` at `:163`, `make_frm_data_and_fraud_check_operation` at `:401`). A v2 merchant enabling FRM panics.

**D7 — Failure semantics are inconsistent and not merchant-configurable.**
- `payments.rs:884-903` — pre-connector FRM error is logged and swallowed (fail-open).
- `fraud_check.rs:527-530` — the Transaction sub-call error is logged and swallowed.
- `fraud_check.rs:516-519` — the Checkout sub-call error propagates with `?` (fail-closed).
- `post_payment_frm_core` propagates.

So a high-risk merchant cannot choose fail-closed, and a low-risk merchant cannot guarantee fail-open. There is also no timeout on FRM calls: a slow provider stalls authorization.

**D8 — Verdict data is lossy.**
`score: Option<i32>` is stored unnormalized (providers use 0–100, 0–1000, and inverted scales interchangeably). `reason: Option<serde_json::Value>` is untyped, so decline-reason analytics are impossible. There is no `Pending` verdict, so genuinely asynchronous providers leave `FraudCheckStatus::Pending` with no resume path or expiry.

**D9 — `fraud_check` is one mutable row per payment.**
Primary key `(frm_id, attempt_id, payment_id, merchant_id)` but the code does `find_fraud_check_by_payment_id_if_present` and updates in place (`fraud_check_pre.rs:99-135`, `fraud_check_post.rs:110-145`). A Pre→Post→Fulfillment sequence overwrites its own history. Shadow-mode comparison and chargeback representment both need the history.

**D10 — Provider registration is bespoke.**
`FraudCheckConnectorData::convert_connector` (`types/api/fraud_check.rs:58-72`) is a hardcoded match over a 3-variant `FrmConnectors` enum (`api_models/src/enums.rs:186`), entirely separate from the main `ConnectorData` machinery.

**D11 — Minor, but symptomatic.**
`is_operation_allowed` (`fraud_check.rs:731`) compares `format!("{operation:?}")` against a string list. Order details are parsed with `.unwrap_or_default()` (`fraud_check.rs:442`), silently dropping malformed data on the exact field a risk engine most depends on. Each of the four flow files re-derives `email`/`phone`/`client_ip` from `payment_intent.customer_details` independently (e.g. `checkout_flow.rs:72-91`).

## 4. Design principles

1. **One context, built once.** Risk facts are assembled by payments core, not per-connector.
2. **Policy is data, not code.** Merchant intent lives in a rule program, reusing Euclid and the existing `routing_algorithm` storage + CRUD.
3. **Providers answer questions; they do not take actions.** A provider returns an assessment. The router decides what to do.
4. **The narrow waist is two verbs.** Everything a risk provider does is either "evaluate this" or "know about this".
5. **Every decision is journaled.** Append-only, including shadow decisions.
6. **v1/v2 differ only in the context builder.**

## 5. Target architecture

```
                    ┌───────────────────────────────────────────────┐
   payments core ──▶│ L1  RiskContext          (facts, versioned)   │
                    └───────────────────┬───────────────────────────┘
                                        ▼
                    ┌───────────────────────────────────────────────┐
                    │ L2  Decision points  (Pre/PostAuth/Capture/…) │
                    └───────────────────┬───────────────────────────┘
                                        ▼
                    ┌───────────────────────────────────────────────┐
                    │ L3  Risk policy      (Euclid program)         │
                    │     → which providers, when, mode, on-failure │
                    └───────────────────┬───────────────────────────┘
                                        ▼
                    ┌───────────────────────────────────────────────┐
                    │ L4  Provider        RiskEvaluate / RiskNotify │
                    │     → RiskAssessment (normalized)             │
                    └───────────────────┬───────────────────────────┘
                                        ▼
                    ┌───────────────────────────────────────────────┐
                    │ L5  Action executor  (owned by payments core) │
                    └───────────────────┬───────────────────────────┘
                                        ▼
                    ┌───────────────────────────────────────────────┐
                    │ L6  risk_decision journal + analytics         │
                    └───────────────────────────────────────────────┘
```

### L1 — RiskContext

New module `crates/hyperswitch_domain_models/src/risk/`.

```rust
pub struct RiskContext {
    pub schema_version: RiskContextVersion,      // V1
    pub decision_point: RiskDecisionPoint,
    pub transaction:  TransactionFacts,   // amount: MinorUnit, currency, capture_method,
                                          // setup_future_usage, is_mit, retry_count
    pub instrument:   InstrumentFacts,    // payment_method, pm_type, card{bin,last4,network,
                                          // issuer,funding,issuer_country}, wallet, token_ref
    pub customer:     CustomerFacts,      // id, email, phone, account_age, prior_success_count
    pub session:      SessionFacts,       // ip, user_agent, accept_language, browser_info,
                                          // device fingerprint
    pub order:        Option<OrderFacts>, // line items, shipping + billing address,
                                          // shipping method, gift indicator
    pub merchant:     MerchantFacts,      // merchant_id, profile_id, mcc, business_country
    pub processing:   ProcessingFacts,    // connector, connector_txn_id, attempt_status,
                                          // avs/cvv result, 3ds outcome, network_txn_id
    pub prior:        PriorRiskFacts,     // decisions already made on this intent,
                                          // blocklist hit, card-testing-guard signal
    pub extensions:   RiskExtensions,
}

/// Namespaced merchant/provider payloads. Replaces the single opaque `frm_metadata`.
pub struct RiskExtensions(HashMap<RiskNamespace, Secret<serde_json::Value>>);
```

Built by `RiskContextBuilder`, which is the *only* place `payment_intent.customer_details` is parsed for risk purposes — removing the duplication at `checkout_flow.rs:72-91` and its three siblings. Adjacent signals contribute through a small trait so FRM need not know about them:

```rust
pub trait RiskFactProvider {
    fn contribute(&self, ctx: &mut RiskContext);
}
// implemented by blocklist, card_testing_guard, 3DS result, debit routing
```

`extensions` is the generic escape hatch that makes this *not strict to a merchant's use case*: a merchant can attach arbitrary namespaced data (loyalty tier, seller ID for a marketplace, device score from their own SDK) and reference it in policy via `DirKeyKind::MetaData` without any Hyperswitch code change.

### L2 — Decision points

Replaces both `FrmPreferredFlowTypes {Pre, Post}` and the linear `FraudCheckLastStep`.

```rust
pub enum RiskDecisionPoint {
    PreAuthorization,   // before the connector call
    PostAuthorization,  // authorized, pre-capture
    PostCapture,
    PreRefund,
    OnFulfillment,
    OnDispute,
}
```

Payments core emits at each point; the policy decides whether anything runs there. `Pre` becomes `PreAuthorization`, `Post` becomes `PostAuthorization` — existing behaviour is a strict subset.

### L3 — Risk policy (Euclid)

Modelled directly on the 3DS decision rule, which is the proven pattern in this repo.

`crates/common_types/src/risk_decision_engine.rs`:

```rust
pub struct RiskDecisionRule {
    pub evaluations: Vec<RiskEvaluationStep>,   // ordered; short-circuits on a terminal action
    pub on_unavailable: FailureMode,            // FailOpen | FailClosed | HoldForReview
}

pub struct RiskEvaluationStep {
    pub provider: common_utils::id_type::MerchantConnectorAccountId,
    pub at: RiskDecisionPoint,
    pub mode: EvaluationMode,                   // Enforce | Shadow
    pub timeout_ms: Option<u32>,
    pub outcomes: OutcomeActionMap,
}

pub struct OutcomeActionMap {
    pub approve: RiskAction,   // default Continue
    pub decline: RiskAction,   // default Reject
    pub review:  RiskAction,   // default HoldForReview
    pub error:   RiskAction,   // default derived from on_unavailable
}

impl EuclidDirFilter for RiskDecisionRule {
    const ALLOWED: &'static [DirKeyKind] = &[
        DirKeyKind::PaymentAmount,      DirKeyKind::PaymentCurrency,
        DirKeyKind::PaymentMethod,      DirKeyKind::PaymentMethodType,
        DirKeyKind::CardNetwork,        DirKeyKind::IssuerCountry,
        DirKeyKind::IssuerName,         DirKeyKind::CardDiscovery,
        DirKeyKind::BillingCountry,     DirKeyKind::BusinessCountry,
        DirKeyKind::CustomerDevicePlatform, DirKeyKind::CustomerDeviceType,
        DirKeyKind::CaptureMethod,      DirKeyKind::SetupFutureUsage,
        DirKeyKind::MetaData,
    ];
}
```

Storage and CRUD are **free**: add `RoutingAlgorithmKind::RiskDecisionRule` (`diesel_models/src/enums.rs:55`, `api_models/src/routing.rs:576`) and store the program in the existing `routing_algorithm` table, referenced by `profile.frm_routing_algorithm_id` — a column that already exists in v2 (`business_profile.rs:626`) and needs adding in v1. Execution mirrors `get_three_ds_decision_rule_output` (`core/three_ds_decision_rule.rs:41`): parse `ast::Program<RiskDecisionRule>`, build `BackendInput` from `RiskContext`, run `VirInterpreterBackend`.

This is the answer to D2 and D3: merchant intent is expressed over facts, not over a fixed `gateway × payment_method` matrix, and it is profile-scoped and multi-provider by construction.

### L4 — Provider abstraction: two verbs

`crates/hyperswitch_domain_models/src/router_flow_types/risk.rs`:

```rust
pub struct RiskEvaluate;   // ask for an assessment; a verdict is expected
pub struct RiskNotify;     // inform the provider of a lifecycle event; verdict optional
```

```rust
pub struct RiskEvaluateRequestData {
    pub context: RiskContext,
    pub reference: Option<RiskReference>,   // provider-side id from an earlier step
}

pub struct RiskNotifyRequestData {
    pub context: RiskContext,
    pub event: RiskLifecycleEvent,          // Authorized | Failed | Captured | Refunded
                                            // | Fulfilled | Chargeback | MerchantDecision
    pub reference: Option<RiskReference>,
}

pub struct RiskAssessment {
    pub reference: Option<RiskReference>,
    pub verdict: RiskVerdict,               // Approve | Decline | Review | Pending | Unavailable
    pub score: Option<RiskScore>,           // { normalized: u16 /*0..=1000*/, raw: f64, scale }
    pub reasons: Vec<RiskReason>,           // { code, category, description }
    pub provider_raw: Option<Secret<serde_json::Value>>,
    pub expires_at: Option<PrimitiveDateTime>,   // for Pending
}
```

Changes versus today, each addressing D1/D8:

- five traits → two;
- `#[serde(untagged)]` variant soup → one struct, killing the 140-line cross-product match in `fraud_check_post.rs:424-563`;
- unnormalized `i32` score → normalized + raw + scale, so a policy threshold means the same thing across providers;
- untyped `serde_json::Value` reason → `Vec<RiskReason>`, which makes decline-reason analytics possible;
- explicit `Pending` + `expires_at`, resolved by a process-tracker workflow (the repo already runs these — see `crates/router/src/workflows/`), rather than a stuck `Pending` row.

**Compatibility shim.** The existing five traits stay. The new core drives them through an adapter using exactly the mapping the current code already implements (verified against `fraud_check_pre.rs:189-265` and `fraud_check_post.rs:180-219`):

| New | Legacy |
| --- | --- |
| `RiskEvaluate` @ `PreAuthorization` | `Checkout` |
| `RiskEvaluate` @ `PostAuthorization` | `Sale` |
| `RiskNotify(Authorized \| Failed)` | `Transaction` |
| `RiskNotify(Refunded)` | `RecordReturn` |
| `RiskNotify(Fulfilled)` | `Fulfillment` |

Signifyd, Riskified and CyberSource Decision Manager are **not modified**. New providers implement only `RiskEvaluate` + `RiskNotify` and register through the standard connector macro path, retiring the bespoke `FrmConnectors` match (D10).

### L5 — Actions, executed by payments core

```rust
pub enum RiskAction {
    Continue,
    ContinueWithManualCapture,             // today's should_continue_capture = false
    Reject { reason: Option<String> },
    HoldForReview,                         // IntentStatus::RequiresMerchantAction
    Capture,
    Refund { amount: Option<MinorUnit> },
    ChallengeWith3ds,                      // hand off to ThreeDSDecision
}
```

A `RiskActionExecutor` owned by payments core interprets these. This deletes the `payments_core::<Void>` / `payments_core::<Capture>` reentrancy at `fraud_check_post.rs:264,321` (D5) and makes the action set extensible by data.

`ChallengeWith3ds` is worth calling out: "step up instead of hard-declining" is a top merchant request and is impossible today. It becomes a one-line policy change because both subsystems now speak decisions rather than side effects.

### L6 — Journal

New append-only table, replacing in-place mutation (D9):

```sql
CREATE TABLE risk_decision (
    id                 VARCHAR(64) PRIMARY KEY,
    merchant_id        VARCHAR(64) NOT NULL,
    profile_id         VARCHAR(64) NOT NULL,
    payment_id         VARCHAR(64) NOT NULL,
    attempt_id         VARCHAR(64) NOT NULL,
    decision_point     VARCHAR(32) NOT NULL,
    provider_mca_id    VARCHAR(64),
    provider_name      VARCHAR(64) NOT NULL,
    mode               VARCHAR(16) NOT NULL,   -- enforce | shadow
    verdict            VARCHAR(16) NOT NULL,
    score_normalized   INT,
    score_raw          JSONB,
    reasons            JSONB,
    action_taken       VARCHAR(32),
    policy_id          VARCHAR(64),
    request_snapshot   JSONB,
    response_snapshot  JSONB,
    latency_ms         INT,
    error_code         VARCHAR(64),
    created_at         TIMESTAMP NOT NULL
);
CREATE INDEX risk_decision_payment_idx ON risk_decision (merchant_id, payment_id);
```

`fraud_check` is retained and kept in sync as the "current state" projection, so the existing `/frm_fulfillment` endpoint, webhook flow and analytics keep working unchanged.

This unlocks three things merchants ask for and we cannot do today: shadow-mode A/B of a challenger provider against the incumbent, per-reason decline analytics, and an auditable trail for chargeback representment.

## 6. Merchant-facing surface

**Onboarding today** (3 steps, merchant-wide, one provider):

1. `POST /accounts/:id` with `frm_routing_algorithm: {"type":"single","data":"signifyd"}` — merchant-scoped.
2. `POST /accounts/:id/connectors`, `connector_type: payment_vas`, with `frm_configs: [{gateway, payment_methods:[{payment_method, flow}]}]`.
3. Expressible policy: *gateway × payment_method → pre|post*. Nothing else.

**Onboarding after** (2 steps, profile-scoped, multi-provider):

1. `POST /accounts/:id/connectors` — credentials only.
2. `POST /routing/risk` → `routing_id`, then `POST /routing/risk/:id/activate` — reusing the existing routing CRUD, including its versioning and activation semantics.

```jsonc
{
  "name": "default risk policy",
  "algorithm": {
    "rules": [
      {
        "name": "high-value cross-border",
        "conditions": [
          { "lhs": "payment_amount",  "comparison": "greater_than", "value": 50000 },
          { "lhs": "billing_country", "comparison": "not_equal",    "value": "US" }
        ],
        "output": {
          "evaluations": [{
            "provider": "mca_signifyd_01",
            "at": "pre_authorization",
            "mode": "enforce",
            "timeout_ms": 2000,
            "outcomes": {
              "decline": { "type": "challenge_with_3ds" },
              "review":  { "type": "hold_for_review" }
            }
          }],
          "on_unavailable": "fail_open"
        }
      }
    ],
    "default_output": { "evaluations": [], "on_unavailable": "fail_open" }
  }
}
```

**New endpoints**

| Endpoint | Purpose |
| --- | --- |
| `POST /payments/:id/risk/decision` | Merchant resolves a `HoldForReview` (approve/decline) — today only reachable via a provider webhook (`incoming.rs:2238`) |
| `GET  /payments/:id/risk` | Decision history for a payment |
| `POST /payments/:id/risk/evaluate` | Out-of-band re-evaluation |

`POST /frm_fulfillment` is retained; it becomes a thin wrapper over `RiskNotify(Fulfilled)`.

## 7. Integrating a new provider — before vs after

| | Today | After |
| --- | --- | --- |
| Enum variant + match arm | `FrmConnectors` + `convert_connector` | not required (standard macro registration) |
| Connector traits | 5 (`Sale`, `Checkout`, `Transaction`, `Fulfillment`, `RecordReturn`) | 2 (`RiskEvaluate`, `RiskNotify`) |
| Transformer flows | 5 | 1 request + 1 response mapping |
| Score/reason normalization | ad hoc per provider | one `RiskScore` / `Vec<RiskReason>` contract |
| Eligibility wiring | edit `should_call_frm` | none — policy is data |
| Provider-specific merchant fields | contend over `frm_metadata` | own namespace in `RiskExtensions` |
| v2 support | not available | free (same core) |

## 8. Migration plan

Each phase is independently mergeable and behaviour-preserving until Phase 3.

| Phase | Content | Behaviour change |
| --- | --- | --- |
| 0 | `RiskContext` + builder; feed the existing five flows from it | none (removes duplication in the 4 flow files) |
| 1 | `RiskAssessment` normalization + `risk_decision` journal written alongside `fraud_check` | none (shadow write) |
| 2 | `RiskEvaluate` / `RiskNotify` traits + compat shim over the legacy five | none (existing providers untouched) |
| 3 | Euclid risk policy + `RoutingAlgorithmKind::RiskDecisionRule`; falls back to `frm_configs` when no policy is set | opt-in per profile |
| 4 | `RiskActionExecutor` in payments core; delete `payments_core` reentrancy from `execute_post_tasks` | none |
| 5 | Decision-point generalization, shadow mode, `Pending` resolution via process tracker | additive |
| 6 | v2 implementation (fills the 14 `todo!()` by construction) | new capability |
| 7 | Deprecate `merchant_account.frm_routing_algorithm` and `mca.frm_configs` | after a deprecation window |

**Back-compat guarantees.** `fraud_check` rows keep being written. `frm_configs` keeps working while no risk policy is set on the profile. `FrmSuggestion`, `FraudCheckStatus`, the `FrmApproved`/`FrmRejected` webhook flow, `/frm_fulfillment`, and the analytics metrics are all unchanged. A merchant who does nothing sees no difference.

## 9. Cross-cutting fixes folded in

- **Fail-open vs fail-closed becomes explicit** (D7) — a single `FailureMode` in policy, applied uniformly, replacing three inconsistent ad-hoc sites.
- **Timeouts** — per-evaluation `timeout_ms`; expiry yields `Unavailable`, which routes through `outcomes.error`. Today a slow provider stalls authorization indefinitely.
- **Order details** — parsed strictly into `OrderFacts` with the failure surfaced, not `.unwrap_or_default()` (`fraud_check.rs:442`).
- **`is_operation_allowed`** — replaced by decision points being emitted only from operations that define them, removing the `format!("{operation:?}")` string comparison (`fraud_check.rs:731`).

## 10. Risks and open questions

1. **Euclid expressiveness.** `DirKeyKind` may lack facts a risk policy wants (BIN lists, velocity counters). Velocity in particular is stateful and does not belong in a stateless interpreter — likely a separate fact provider feeding a boolean/bucketed value into the context.
2. **Journal volume.** One row per evaluation per attempt, plus shadow rows. Needs a retention policy and probably the same KV/drainer treatment as other hot tables.
3. **Shadow-mode cost.** Shadow evaluations are billable provider calls. Must be opt-in and rate-limitable.
4. **`ChallengeWith3ds` ordering.** At `PreAuthorization` this interacts with the existing 3DS decision rule; the precedence between a risk-driven step-up and an existing `ThreeDSDecision` needs to be pinned down before Phase 5.
5. **UCS.** There is no FRM service in `proto/`. If risk providers eventually move to the gRPC connector service, `RiskEvaluate`/`RiskNotify` are the right proto surface — worth confirming direction before Phase 2 hardens the traits.
