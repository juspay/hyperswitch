---
name: payment-routing-reviewer
description: Project knowledge and review checklist for the Techdome take-home - a cost-aware payment routing layer in front of Hyperswitch. Use when implementing, extending, testing or reviewing the cost-router, its fee/stats/trace code, its tests, or DECISIONS.md. Separates verified repo facts from assumptions and unresolved decisions.
---

# Payment routing reviewer (cost-aware routing over Hyperswitch)

Status legend used throughout:
- **[V]** verified by reading this repo (file:line given; lines drift, re-grep before relying on them)
- **[A]** assumption, not verified
- **[?]** open question, to be settled by running the stack or by the owner
- **[APPROVED]** decided with the owner. **[PROPOSED]** discussed, NOT approved - never implement as a fixed requirement.

## 1. Assignment (Techdome Venture Studio, Senior/Staff take-home)
Fork Hyperswitch, add ONE non-trivial, measurable custom routing logic. Budget 1-2 h, 48 h window. Deliverables: public repo, <=3 min Loom, self-scorecard, DECISIONS.md (<=1 page, honest about what is broken/hacky and what 4 more hours would buy), committed `.claude/`, Claude Code session transcript, README so a new engineer sends a test payment in <10 min (`make demo` preferred).

Definition of done: spin up <10 min; send a test payment (curl/script); see the custom logic fire in logs or a trace WITH its inputs and the chosen connector; tests run with ONE command (happy path + edge case + failure-mode); DECISIONS.md.

Scorecard (100): working demo 25, routing depth 20 (clear decision boundary, degrades gracefully), tests 15 (happy + edge + failure mode), code quality 10 (no copy-paste, types/errors handled), decision doc 10, Claude Code leverage 10, README 10. Bonus up to +15: `GET /v1/payments/{id}/routing-trace` (+5), Prometheus/OTel metrics per connector (+5), load test 100 RPS x 60 s p99 < 200 ms on the custom path (+5).

Auto-fail: demo not running from README in 15 min; no tests; logic is a hardcoded `if currency == "INR"` with no abstraction; AI slop (hallucinated APIs, dead imports, comments not matching code); late. Tiebreakers: spec before code (visible in commit history), knowing what to skip, visibly rejecting bad Claude output, honest self-assessment, one visible production concern (idempotency / retry storms / PII in logs).

## 2. Chosen strategy
**[APPROVED]** Cost-aware routing that weighs processing fee AND reliability (cheapest-only is rejected: a 96%-reliable Rs50 PSP can beat an 85% Rs20 PSP).
**[APPROVED]** Implement as a NestJS/TypeScript layer ("cost-router") in a `cost-router/` directory of the fork, in front of prebuilt Hyperswitch Docker images. Reason: the dev machine (8 GB RAM, no Rust toolchain) cannot realistically compile the Rust router. State this honestly in DECISIONS.md. The decision engine must be a PURE, framework-free module (no Nest/HTTP/Redis imports) so it can later be ported into Rust (`RoutingStage`) unchanged in logic.
**[APPROVED]** Build in small parts: (1) basic setup, (2) engine skeleton, (3) first rule, (4) next rule. Nothing is committed until the owner says so.
**[?]** Founders have not confirmed that a sidecar using Hyperswitch's `routing` override satisfies "add custom routing logic"; the brief rewards asking.

## 3. Hyperswitch facts (all [V] unless marked)
Extension points (v1 feature, `crates/router/src/core/payments/`):
- `payments.rs` `decide_connector` (~12502): pre-decided connectors (sync/cancel flows) bypass routing; a request-level straight-through algorithm is preferred over merchant-configured algorithms (~12589-12603); routing failure falls back to the profile's default connector list (`RoutingApproach::DefaultFallback`); result goes to `plan_payment_execution_after_routing`.
- `routing.rs`: `RoutingStage` trait with `StraightThroughRoutingStage` (~775-800). `perform_straight_through_routing` (~2124): `Single` -> one connector; `Priority(list)` -> list, eligibility analysis required; `VolumeSplit` -> weighted pick. A comment (~1378) says `Single` has nothing to fail over to, but see section 13: with profile auto-retry ON, a live run failed over past a `single`/short list anyway.
- Request API: `routing: Option<serde_json::Value>` and `connector: Option<Vec<Connector>>` in `crates/api_models/src/payments.rs` (~1285, ~1290). **[V live run]** The priority shape `{"type":"priority","data":[{"connector":"<name>","merchant_connector_id":"<id>"}]}` is accepted by POST /payments (serde: `StraightThroughAlgorithm`, `crates/api_models/src/routing.rs`). A malformed list gives 400 `IR_06`.
- Enums for a future Rust port: `api_models/src/routing.rs` `RoutingAlgorithmKind` (~576), `StaticRoutingAlgorithm` (~629); `diesel_models/src/enums.rs` (~65, needs a migration); `common_enums` `RoutingApproach` (~12127). `routing_approach` is stored on payment attempts.
- What does NOT exist: no PSP fee/cost concept in `routing.rs`. `core/debit_routing.rs` does network-level cost for co-badged US debit via an external service - it is not PSP fee ranking.

Retry / fallback (`retry.rs`):
- Retry is driven by `do_gsm_actions` (~44): on a failed attempt it looks up a GSM (gateway status map) record for (connector, error code); decision `Retry` or `DoDefault`.
- Auto-retry is OFF unless `max_auto_retries_enabled` is set on merchant config or profile; `retries None/0` stops (~164). Connectors exhausted also stops (~170).
- The next connector is popped from the connector list computed ONCE at routing time (`connector_routing_data`, ~170/218). Fallback does NOT re-evaluate routing. So the order of our priority list IS the failover order.
- Each retry creates a new `payment_attempt` (same `payment_id`, new `attempt_id`, `retry_type = AutoRetry`) via `make_new_auto_retry_payment_attempt` (~821). Special paths: 3DS step-up, MIT/off-session (uses next connector with mandate handling), clear-PAN retry on same connector.
- **[?]** how timeouts/ambiguous connector responses interact with retry (double-charge risk): NOT tested (the dummy connector cannot simulate a timeout).
- **[V live run]** a fresh install has no GSM retry rows; without a matching row a failed payment ends `failed` on the first connector.
- Duplicate protection: repeating a merchant-supplied `payment_id` yields `DuplicatePayment` (`operations/payment_create.rs` ~384-418).

Data model facts:
- Amounts are `MinorUnit` (paise/cents) in the API. `PaymentAttemptResponse` (api_models/payments.rs ~2194) has `status`, `amount`, `currency`, `connector`, `error_message`; payment responses carry `attempts: Option<Vec<PaymentAttemptResponse>>`; `expand_attempts` exists on retrieve requests (~8971, 8993, 10387). **[V live run]** the CREATE response does NOT include `attempts`; `GET /payments/{id}?expand_attempts=true` does.
- `AttemptStatus` (common_enums ~151): Started, AuthenticationFailed, RouterDeclined, AuthenticationPending, AuthenticationSuccessful, Authorized, AuthorizationFailed, Charged, Authorizing, CodInitiated, Voided, VoidedPostCharge, VoidInitiated, CaptureInitiated, CaptureFailed, VoidFailed, AutoRefunded, PartialCharged, PartiallyAuthorized, PartialChargedAndChargeable, Unresolved, Pending, Failure, PaymentMethodAwaited, ConfirmationAwaited, DeviceDataCollectionPending, IntegrityFailure, Expired, CaptureReview.
- Hyperswitch analytics (success-rate metrics, routing events) exist but need an OLAP store (ClickHouse); too heavy/slow for this. **[A]** built-in success-based routing depends on an external decision-engine service.
- **[V live run]** the prebuilt `:standalone` image supports dummy connectors `stripe_test`, `fauxpay`, `pretendpay`. Local env: Docker Desktop on WSL2 (capped 4 GB).

## 4. Fee calculation (rules are [PROPOSED] unless marked)
- **[APPROVED]** All money is integer minor units. Never use floats for money. Use integer math (BigInt or guarded safe-integer ops) with an explicit rounding rule (state it; round half up is the proposal). `amount * bps` can approach 2^53 for very large amounts - guard it.
- Proposed fee: `clamp(round(amount * bps / 10000) + fixed, min, cap)`; bps = basis points (100 bps = 1%).
- Match rules most-specific-first (currency, optional BIN class / card type / amount band). Zero fees are valid (e.g. zero-MDR UPI/RuPay debit) - no division by fee anywhere.
- Per-transaction "amount slabs" are mostly artificial (real tiers are monthly-volume); the real amount effect is the fixed fee. Keep band matching optional.
- Missing pricing for an otherwise working connector: **[PROPOSED]** do not make payments fail; rank it last and flag it in the trace (open question 6 below). Invalid pricing (negative values, cap < min, non-integers, duplicate rules): fail fast at startup validation, not per request.
- Cross-currency/FX settlement, failed-attempt fees, volume tiers: deferred.

## 5. Reliability / statistics
- **[APPROVED]** Measure per ATTEMPT, not per final payment outcome. If A fails and Hyperswitch retries to B which succeeds, the payment is "succeeded" but A must still be charged a failure. Source: the `attempts` list.
- Counting rule **[PROPOSED]**: success = Charged/Authorized; failure = Failure/AuthorizationFailed (split technical failures - timeouts, 5xx, config errors - from issuer declines, which should not penalise a connector); Pending/Authorizing/AuthenticationPending/ConfirmationAwaited = unresolved, not counted until resolved. Whether decline codes are distinguishable from responses is **[?]**.
- Segment by (connector, currency) at minimum; rolling window of time buckets. Window length and minimum sample size are config. Small samples are noisy (9/10 can be 60%): prefer a Bayesian shrink toward the peer mean or a Wilson lower bound over raw rates **[PROPOSED]**; a pure lower bound starves new connectors (cold-start deadlock).
- Own in-memory tracking behind a small interface **[APPROVED as MVP]**; no Redis, no circuit breaker in MVP. Known limits: state lost on restart, per-replica divergence.

## 6. Scoring model
**[APPROVED - MVP]** (owner decision) single-connector score: `score = s_hat * fee + (1 - s_hat) * L_eff`, lowest wins, same order is the fallback order. `L_eff` = net expected cost of a failed first attempt after fallback, configured as `failure_penalty_pct` (percent of the payment amount). `failure_penalty_pct` is REQUIRED config, validated, with NO default and no invented value. This is a single-connector approximation, NOT the optimal fallback-chain calculation when retries are enabled; the root DECISIONS.md must say so. L_eff is clamped to at least the highest candidate fee (flagged in the trace). Trace carries score components, `s_hat`, observed attempts, fee, effective failure cost, final score, eligibility, selection reason, and break-even penalty `L* = (s1*f1 - s2*f2)/(s1 - s2)` for chosen vs runner-up where defined.
**[APPROVED]** Missing pricing: unpriced connectors ranked last and flagged; if NO connector has usable pricing, omit the override and let Hyperswitch route normally.
**[APPROVED]** Shrinkage `s_hat = (successes + k*prior)/(n + k)` with k = min sample config, only over defined outcome classes (section 5 table in cost-router docs). Pending/unresolved attempts are never failures.
**[NEEDS OWNER APPROVAL]** absolute `min_success_rate` guard (brief asks for a floor); classification of unclassified failures as technical. No relative floor, no tie-percentage margin.
Background (discussed earlier):
- Single-attempt score `s*fee + (1-s)*L` (fee charged only on success).
- Chain view: `E(chain) = s1*f1 + (1-s1)*[h + rho*E(rest) + (1-rho)*L]`; rho = recoverable share of failures (issuer declines are not recoverable), h = friction of a recovered failure. rho=0 is the single-attempt formula; with rho=1, h=0 cheapest-first always wins for 2 PSPs. With retries off (Hyperswitch default) the chain has length 1.
- Proposed MVP collapse: `score = s*fee + (1-s)*L_eff`, `L_eff = h + (1-rho)*M_lost + rho*fallback_fee`, sorted ascending; L must exceed every fee (clamp and warn).
- Worked example (retries off): A (Rs50, 96%), B (Rs20, 85%): tie at L ~ Rs282; A wins above, B below.
Open: see section 10. The failure-cost parameter has NO default - do not invent one (no "25%").

## 7. Priority list integration
**[PROPOSED]** The layer returns a ranked list and sends `routing: {type: priority, data: [...]}` so existing retry walks it in order. `Single` removes failover. Usable fallbacks are bounded by the configured retry count. **[V live run]** the first attempt follows the list order (four orders tested), but the list is a PREFERENCE, not a guarantee (section 13). If the layer errors, omit the override so Hyperswitch routes normally - a payment must never fail because of this layer. Idempotency: require/derive a stable `payment_id` and persist the decision per payment so a client replay gets the same decision.

## 8. Decision trace / observability
Per decision record (never a PAN; BIN prefix only): payment id, inputs (BIN class, currency, minor-unit amount), per-candidate fee, success rate + sample size, eligibility/exclusion reason, score components if a scorer is used, chosen order, degradation flags, timestamp, and the realized outcome (connector that succeeded, realized fee, savings vs baseline default order). Log it structured. Bonus: `GET /v1/payments/{id}/routing-trace`, Prometheus counters per connector.

## 9. Test requirements
Pure-engine unit tests (no network): fee math (rounding, min/cap, zero fee, big amounts), config validation (negative, cap<min, duplicates), missing pricing, insufficient/no stats, eligibility exclusion, deterministic ordering and tie handling, trace contents. Scenario tests: cheapest wins when reliability is comparable; pricier reliable connector wins when justified; first connector fails and fallback succeeds (attempt-level stats credit the right connector); all connectors fail; layer failure -> override omitted. One command runs everything. Do not write tests that assert unapproved constants.

## 10. Open decisions (items 1-6 settled for the MVP by section 6; 7 still needs the running stack)
1. Independent per-connector score vs ordered-chain expected cost (and is the permutation search worth it).
2. How a recovered failure is priced (h, fallback fee).
3. What failure cost L means (lost margin? order value share?) and where its value comes from.
4. Reliability estimate with few attempts (shrinkage vs lower bound vs neutral prior).
5. Which reliability safeguards are truly needed (absolute floor yes/no, relative gap yes/no).
6. Missing pricing: excluded vs ranked last.
7. Proving the priority list survives eligibility analysis and retry (needs the running stack).

## 11. Deferred / out of scope (do not add unless asked)
Circuit breaker, Redis-backed stats, warm-up/exploration quotas, latency-aware routing, BIN/issuer-level stats, FX settlement, failed-attempt fees, volume tiers, Rust port, Euclid/DSL integration, other routing strategies.

## 12. Review checklist
- Money is integer minor units end to end; no float arithmetic on amounts.
- Engine imports no framework/HTTP/storage code; same input -> same output (inject clock/randomness; no `Math.random`).
- Every unapproved constant is config or a TODO, not a literal.
- No fabricated Hyperswitch API: every field/endpoint used is traceable to a file in section 3 or marked [A]/[?].
- Existing Hyperswitch files untouched unless a change is justified in DECISIONS.md.
- Failure modes: bad config fails at startup; runtime errors degrade to "omit override"; no PAN in logs; idempotent per payment id.
- No dead imports, no comments that contradict code, no duplicated fee/ordering logic.
- Nothing committed without the owner's approval.

## 13. Verified in a live run (2026-11-17, prebuilt `docker.juspay.io/juspaydotin/hyperswitch-router:standalone` v1.129.0)
Evidence and commands: `cost-router/docs/INTEGRATION_RESULTS.md`; reproduce with `cost-router/scripts/up.sh` and `demo.sh`.
- Priority `routing` accepted; first attempt follows the list (tested [fauxpay,stripe_test], [stripe_test,fauxpay] and 3-connector lists).
- **Forced routing is not strict.** A disabled, ineligible or unknown connector (or a bogus `merchant_connector_id`) is silently replaced, no error. Entries without `merchant_connector_id` are ignored. Always compare the response `connector` and `attempts` with the request.
- **The list is not a whitelist once auto-retry is on.** Retries continued to a connector not in the list; `single` is not "no failover". Only profile `is_auto_retries_enabled: false` stopped failover (no per-request switch found). A connector excluded by our reliability floor can therefore still run on a retry.
- Auto-retry needs the profile (`is_auto_retries_enabled: true`, `max_auto_retries_enabled: N`, N=2 gave up to 3 attempts) AND a GSM rule with `decision: retry`. GSM key: `flow:"Payment"`, `sub_flow:"Authorize"` (capital A; `authorize_flow` silently misses), plus the exact connector `code` and `message`.
- `attempts[]` (expanded) has `attempt_id`, `status`, `connector`, `error_code`; NO `merchant_connector_id`; order not guaranteed; expanded `error_message` was null. So the trace claims no attempt ordering.
- A replayed merchant `payment_id` is rejected: 400 `HE_01`.
- Dummy connector fixtures: card `4242424242424242` succeeds everywhere; `4000000000009995` fails on fauxpay/pretendpay (`DC_08`) and succeeds on stripe_test (deterministic first-fails-last-succeeds); `4000000000000002` declines everywhere.
- Setup quirks: the `:standalone` image panics on the repo's current `config/docker_compose.toml` (unknown connector), so a trimmed config copy lives in `cost-router/docker/config/`; the router needs `superposition`; `migration_runner` applies migrations then reports an unrelated patch error; admin auth header is `Authorization: admin-api-key=test_admin`; connector and profile calls use the merchant `api-key`.
- Shrinkage prior: a leave-one-out (peers only) prior inverted the ranking with thin data (a connector with one success ranked below one that had only failed). The engine now uses ONE common pooled prior over all distinct candidates (`pooledPrior`).
- Trace: `routing.execution` records what Hyperswitch actually did (final connector/status, attempts listed alphabetically, connectors used outside the requested list) next to the requested `decision`.
