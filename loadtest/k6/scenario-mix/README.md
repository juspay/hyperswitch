# Scenario-mix load test

A single k6 script (`scenario-mix.js`) that runs **all** Hyperswitch payment
scenarios at once, with a configurable percentage of the total traffic for
each — for example 50% guest, 40% modular CIT off-session, 10% metadata-changed.

The scenario catalogue and request semantics are ported from the
`loadtest/runner` automation (`runner/lib/scenarios.js`, `runner/k6/*.js`),
with one deliberate difference: instead of a fixture stage followed by
confirm-only measured traffic, **every iteration is self-contained** — it
creates everything it needs (customer, payment-method session, saved card,
payment) inline and times every step. No pre-created fixtures, no runner
state files, no deployment tooling required; all you need is k6, a running
Router (and payment-method service for customer/modular scenarios), and
merchant credentials.

## Quick start

```bash
cd loadtest/k6/scenario-mix
cp config.example.json config.json      # config.json is git-ignored
# edit config.json: services, merchant credentials, scenario weights
k6 run scenario-mix.js
```

Options:

```bash
# Use a config anywhere on disk:
SCENARIO_MIX_CONFIG=/path/to/config.json k6 run scenario-mix.js

# Also write the full k6 summary JSON to a file — lands in ./output/
# (the default OUTPUT_DIR, already checked into the repo) unless overridden:
SUMMARY_OUTPUT=summary.json k6 run scenario-mix.js

# Stream raw metrics for per-failure-reason analysis:
k6 run --out json=metrics.json scenario-mix.js

# Suppress the periodic progress output (see note below), keep only the
# final summary table:
k6 run -q scenario-mix.js

# Route SUMMARY_OUTPUT somewhere other than ./output/ (create it first —
# see "Collecting everything into one output directory"):
mkdir -p out/run1 && OUTPUT_DIR=out/run1 SUMMARY_OUTPUT=summary.json k6 run scenario-mix.js

# Live/exported time-series charts (throughput, latency over time) instead
# of the end-of-run-only view above — see "Visualizing throughput/latency
# over time":
k6 run -q --out 'web-dashboard=export=timeseries.html' scenario-mix.js
```

Config/validation errors are reported before any traffic is sent.

**Progress output in ramp mode:** every phase of every scenario is its own
k6 executor, and k6's terminal progress bar lists *all* of them each tick —
including future phases shown as `waiting`. With more than a handful of
phases this easily exceeds what a terminal can redraw in place, so instead
of updating in place k6 falls back to reprinting the whole listing every
tick, which looks like runaway scrolling. This is k6's own renderer, not
something `scenario-mix.js` controls; use `-q`/`--quiet` above to turn it
off, or pair it with the web dashboard (`--out web-dashboard`, see
*Visualizing throughput/latency over time*) for a live view that isn't
constrained by terminal height at all.

## How the traffic split works

Each entry in `scenarios` becomes its own k6
[`constant-arrival-rate`](https://grafana.com/docs/k6/latest/using-k6/scenarios/executors/constant-arrival-rate/)
executor with:

```
rate = load.total_rps * weight / 100   (iterations per second)
duration = load.duration_seconds       (same for every entry)
```

Rules:

- `weight` is a percentage of the total; **all weights must add up to 100**
  (checked at startup).
- `weight: 0` disables the entry — handy for shrinking the mix without
  deleting it from the file.
- Entry `name` must be unique and contain only letters, digits and
  underscores; it becomes part of the generated metric names.
- Fractional rates work: they are automatically expressed with a longer
  k6 `timeUnit` (e.g. 0.5 iterations/s becomes `rate: 1, timeUnit: "2s"`),
  since `constant-arrival-rate` requires an integer iteration count.

Because each entry has its own executor and its own set of metrics, the split
is exact (deterministic scheduling) and every percentile is reported per
entry — no random sampling and no post-processing needed. In ramp mode
(`load.phases`), the same weights are applied at every RPS level, so the
relative split stays constant throughout the ramp.

## Test types

The script is a multi-purpose harness — the configuration decides which
[k6 test type](https://grafana.com/docs/k6/latest/testing-guides/test-types/)
you are running:

| Test type | Config pattern |
| --- | --- |
| Smoke | Flat, tiny `total_rps`, a few seconds |
| Average-load | Flat at expected production RPS with your production mix, 5–60 min |
| Stress | Flat, or a few phases slightly above expected peak with holds |
| Spike | Phases with a large `step_rps` and ~0 `idle_seconds` (sudden jump between levels) |
| [Breakpoint](https://grafana.com/docs/k6/latest/testing-guides/test-types/breakpoint-testing/) | Ramp with `target_rps` set unrealistically high until failure appears — the stepped variant; see *Finding maximum RPS* |
| Soak | Flat moderate RPS for hours |

## Traffic-split example

Example: 3 scenarios

```json
"load": { "total_rps": 10, "duration_seconds": 60 },
"scenarios": [
  { "name": "nomod_guest",  "merchant_path": "non_modular", "scenario": "guest",           "weight": 50 },
  { "name": "mod_cit_off",  "merchant_path": "modular",     "scenario": "cit_off_session", "weight": 40 },
  { "name": "ptv_on",       "merchant_path": "modular",     "scenario": "ptv_on_session",  "weight": 10 }
]
```

→ `nomod_guest` at 5 it/s, `mod_cit_off` at 4 it/s, `ptv_on` at 1 it/s, all
for 60 seconds.

## Scenario catalogue

`merchant_path` selects the request flow common to every scenario:

- `non_modular`: payment create → payment confirm (Router only).
- `modular`: PM session create → payment create → PM session confirm →
  payment confirm with the returned `payment_method_token`.

Quick reference:

| `scenario` | Allowed `merchant_path` | Customer | Save-card (`setup_future_usage`) | Session storage | Requests (non_modular / modular) |
| --- | --- | --- | --- | --- | --- |
| `guest` | both | no | — | volatile | 2 / 4 |
| `cit_on_session` | both | yes | `on_session` | persistent | 3 / 5 |
| `cit_off_session` | both | yes | `off_session` | persistent | 3 / 5 |
| `ptv_on_session` | modular only | yes | `on_session` | volatile → persistent after authorization | — / 5 |
| `ptv_off_session` | modular only | yes | `off_session` | volatile → persistent after authorization | — / 5 |
| `cit_metadata_changed` | non_modular only | yes | `off_session` | persistent + pre-saved card per iteration | 5 (+ baseline) / — |
| `sdk_checkout` | non_modular only | no | — | volatile | 5 / — |

### `guest`

One-off checkout, no account, nothing saved for later. No `customer_id`, no
`setup_future_usage`, no `customer_acceptance` anywhere in the request
bodies. Exercises the minimal-write auth path — no customer row, no vault
write — and is the cheapest, fastest scenario, useful as a latency floor to
compare the others against.

- `non_modular`: `payment_create` → `payment_confirm`.
- `modular`: `pm_session_create` (`storage_type: volatile`) → `payment_create`
  → `pm_session_confirm` → `payment_confirm`.

### `cit_on_session`

Customer-initiated save-card, where the customer is present/interactive for
the *next* charge too (e.g. "remember my card" on a storefront). The card is
vaulted **inline**, as part of the session-confirm/payment-confirm call
itself — not after the fact.

- `non_modular`: `customer_create` → `payment_create` (`customer_id`,
  `setup_future_usage: on_session`) → `payment_confirm` (with
  `customer_acceptance`).
- `modular`: `customer_create` → `pm_session_create`
  (`storage_type: persistent`) → `payment_create` → `pm_session_confirm`
  (with `customer_acceptance`, since `setup_future_usage` is set) →
  `payment_confirm`.

### `cit_off_session`

Same save-card mechanics as `cit_on_session`, but for merchant-initiated
charges where the customer won't be present (subscriptions, recurring
billing). The request shape is identical to `cit_on_session` — the only
difference is the `setup_future_usage: "off_session"` value, a
consent/SCA-exemption flag that changes how Router and the connector treat
future charges against the card, not how this script builds the request.

### `ptv_on_session` / `ptv_off_session` — modular only

"Payment to Vault": the opposite ordering from `cit_*`. The session is
created **volatile** (the card lives only transiently, e.g. in Redis,
through the confirm) instead of being vaulted inline. Only *after* the
payment successfully authorizes does the platform promote the card into
persistent storage — "save only if the charge succeeds," vs. `cit_*`'s "save
up front, then charge." The request shape looks almost identical to
`cit_*_session`'s modular path; what differs is the server-side code path
`storage_type: volatile` triggers (auth-then-promote instead of
vault-then-auth). Config validation rejects `merchant_path: non_modular` for
these — there is no non-modular equivalent.

- `modular`: `customer_create` → `pm_session_create`
  (`storage_type: volatile`) → `payment_create` (`setup_future_usage` set) →
  `pm_session_confirm` (with `customer_acceptance`) → `payment_confirm`
  (`setup_future_usage` + `customer_acceptance`).

### `cit_metadata_changed` — non_modular only

Updates a saved card's metadata (new expiry, corrected holder name) while
keeping the *same PAN* — e.g. a card network reissuing an expiry date on an
existing number. Exercises the vault's "recognize this is the same card,
replace metadata" dedup logic rather than "create a new vault entry."

This is the only scenario with a two-stage iteration:

1. **Baseline (unmeasured prep, every iteration):** `customer_create` →
   `baseline_create` → `baseline_confirm` (confirms with the *original*
   card, `setup_future_usage: off_session`, saving it) → polls
   `GET /payments/{id}` up to 50×/5s until `payment_method_id` appears,
   since card persistence can complete slightly after the confirm response
   returns.
2. **Measured:** `payment_create` → `payment_confirm`, resubmitting the
   **same `card_number`** but with `payment.metadata_update` fields (default:
   new `card_exp_month`/`card_exp_year`/`card_holder_name`) merged over it.
   Only this stage counts toward the confirm-latency percentiles — the
   baseline stage has its own separate Trends (`baseline_create_ms_*`,
   `baseline_confirm_ms_*`).

**Correctness risk:** every VU running this scenario reuses `payment.card`
(the same PAN) for its baseline save unless `payment.card_pool` has multiple
cards. Concurrent baseline saves of the same PAN can race in the vault's
dedup logic — keep this scenario's weight low, and/or configure several
distinct test PANs in `card_pool` (each iteration picks one round-robin).

### `sdk_checkout` — non_modular only

Replicates the calls a real Hyperswitch Web/Mobile SDK integration makes
between creating the payment intent and confirming it — traffic shapes the
other scenarios don't exercise at all. No customer, no saved card; the
measured confirm at the end is a plain guest-style card charge.

- `payment_create` → `payment_method_list` (`GET /payments/{id}/client`) →
  `session` (`POST /payments/session_tokens`) → `eligibility`
  (`POST /payments/{id}/eligibility`) → `payment_confirm`.
- `payment_method_list` lists the payment methods available for this payment
  session (no body).
- `session` fetches wallet SDK session tokens; the wallet list comes from
  `sdk.wallets` in config (default `[]` — most test merchants have no wallet
  connectors configured, and an empty list is a valid request).
- `eligibility` runs a BIN-based eligibility/surcharge check against the same
  card (`payment.card`/`card_pool`) the measured confirm will submit.
- These three calls authenticate with a base64-encoded **SDK Authorization**
  header instead of the merchant `api-key` — see *SDK Authorization header*
  below. This is the same header a real SDK sends, so `merchant.publishable_key`
  is required whenever `sdk_checkout` is enabled (same requirement as the
  modular path, for a different reason).
- Non-modular merchant path only — the v2/modular payment-method service
  doesn't expose 1:1 equivalents of `payment_method_list`/`session`/`eligibility`
  yet.

### SDK Authorization header

`payment_method_list`, `session`, and `eligibility` are authenticated the way
the Hyperswitch SDK actually authenticates: an `Authorization` header holding
base64-encoded (standard, padded), comma-separated `key=value` pairs —
`profile_id=...,publishable_key=...,client_secret=...,payment_id=...` — built
fresh per iteration from `merchant.profile_id`, `merchant.publishable_key`,
and the `payment_id`/`client_secret` returned by that iteration's
`payment_create`. Router decodes this in `SdkAuthorization::decode`
(`crates/hyperswitch_domain_models/src/sdk_auth.rs`); k6's `k6/encoding`
`b64encode` (default alphabet) produces the same encoding as Rust's standard
base64 engine used there.

## What one iteration does

Steps are executed in order; each is timed into its own per-scenario Trend.
Any failure stops the iteration and increments the scenario's failure
counter with a `reason` tag.

| Step | When | Endpoint |
| --- | --- | --- |
| `customer_create` | scenario requires a customer (always via the PM service, even for `non_modular` CIT) | `POST {modular_pm}/customers` |
| `pm_session_create` | modular path | `POST {modular_pm}/payment-method-sessions` (with the scenario's `storage_type`) |
| `baseline_create` + `baseline_confirm` | `cit_metadata_changed` only | `POST {router}/payments` + `/payments/{id}/confirm` saving the card `off_session`, then polls `GET /payments/{id}` until `payment_method_id` appears |
| *(think time)* | `load.think_time_ms > 0` | sleep between preparation and measured requests |
| `payment_create` | always | `POST {router}/payments` with `confirm: false` |
| `payment_method_list` | `sdk_checkout` only | `GET {router}/payments/{id}/client` (SDK Authorization header) |
| `session` | `sdk_checkout` only | `POST {router}/payments/session_tokens` with `wallets` from `sdk.wallets` (SDK Authorization header) |
| `eligibility` | `sdk_checkout` only | `POST {router}/payments/{id}/eligibility` with the card (SDK Authorization header) |
| `pm_session_confirm` | modular path | `POST {modular_pm}/payment-method-sessions/{id}/confirm` with the card; adds `customer_acceptance` when the scenario sets `setup_future_usage` |
| `payment_confirm` | always | `POST {router}/payments/{id}/confirm` — modular: `payment_token`; non-modular: card data — plus `setup_future_usage` + `customer_acceptance` when applicable |

A measured confirm counts as **success** when HTTP status is 2xx and the
payment status is one of `succeeded`, `requires_capture`, `processing`.

`cit_metadata_changed` specifics: the measured confirm resubmits the same PAN
but overrides the fields found in `payment.metadata_update`
(default: new expiry month/year and holder name), exercising vault metadata
replacement. Concurrent baseline saves of the **same PAN** can race in vault
deduplication — keep this scenario's weight low and/or use `payment.card_pool`
with several distinct test PANs (each iteration picks one round-robin).

## Configuration reference

`SCENARIO_MIX_CONFIG` selects the file (default `./config.json`, resolved by
k6); the file is plain JSON — no comments allowed.

### `services` (required)

| Key | Meaning |
| --- | --- |
| `router` | Router base URL, e.g. `http://127.0.0.1:8080` or `https://sandbox.hyperswitch.io` |
| `modular_pm` | Payment-method service base URL **including API prefix**, e.g. `http://127.0.0.1:8081/v2` locally or `https://sandbox.hyperswitch.io/v1` behind the sandbox ingress. Required unless every enabled entry is a `non_modular` guest. |

### `target_headers` (optional)

Extra headers merged into every request per service — for routing a shared
ingress and for asking Router to report its internal latency:

```json
"target_headers": {
  "router":      { "x-feature": "sandbox-custom-c3", "x-hs-latency": "true" },
  "modular_pm":  { "x-feature": "sandbox-pm-loadtest" }
}
```

### `merchant` (required)

Existing-merchant credentials; nothing is provisioned or mutated.

| Key | Required when |
| --- | --- |
| `api_key` | always (Router `api-key` header; PM service `Authorization: api-key=...`) |
| `profile_id` | always (sent on every payment create and as `x-profile-id` on PM calls) |
| `publishable_key` | at least one enabled entry uses the modular path (session confirm auth: `Authorization: publishable-key=...,client-secret=...`), or `sdk_checkout` is enabled (SDK Authorization header, see above) |

### `load`

Two mutually exclusive load shapes — flat or a stepped ramp:

**Flat:**

| Key | Default | Meaning |
| --- | --- | --- |
| `total_rps` | — (required) | Total iterations per second across all entries |
| `duration_seconds` | — (required) | How long every entry's executor runs |

**Stepped ramp** (`phases`, cannot be combined with `total_rps`/`duration_seconds`;
same knob names and semantics as `loadtest/runner`'s load phases):

| Key | Default | Meaning |
| --- | --- | --- |
| `phases.starting_rps` | — (required) | Total RPS of the first phase |
| `phases.target_rps` | `starting_rps` | Total RPS of the last phase |
| `phases.step_rps` | `0` | RPS increment between phases (required when ramping) |
| `phases.hold_seconds` | — (required) | How long each phase is held |
| `phases.idle_seconds` | `0` | Gap with no traffic between phases |

**Shared:**

| Key | Default | Meaning |
| --- | --- | --- |
| `request_timeout_ms` | `30000` | Per-request timeout |
| `think_time_ms` | `0` | Pause between the preparation steps and the measured requests |
| `pre_allocated_vus_multiplier` | `3` | `preAllocatedVUs = ceil(rate × this)` per entry (per phase in ramp mode) |
| `max_vus_multiplier` | `10` | `maxVUs = ceil(rate × this)` per entry (per phase in ramp mode). Generous on purpose: iteration duration balloons at saturation, which is exactly when extra VUs are needed. It is a ceiling, not a reservation. |

Each iteration issues up to ~6 sequential requests, so a VU is busy much
longer than `1/rate`. If k6 reports `dropped_iterations`, raise the VU
multipliers.

### `payment`

| Key | Default | Meaning |
| --- | --- | --- |
| `amount` | `1000` | Payment amount (minor units) |
| `currency` | `USD` | Payment currency |
| `session_expiry` | `900` | Payment/PM session expiry in seconds |
| `card` | — (required) | Test card used everywhere (`card_number` required) |
| `card_pool` | `[card]` | Optional array of additional cards; each iteration picks one round-robin. Recommended for `cit_metadata_changed`. |
| `metadata_update` | — | Fields merged over the card for the measured `cit_metadata_changed` confirm. Required when that scenario is enabled. |

### `sdk` (optional)

| Key | Default | Meaning |
| --- | --- | --- |
| `wallets` | `[]` | Wallet `PaymentMethodType`s requested by `sdk_checkout`'s `session` call (e.g. `["google_pay", "apple_pay"]`). An empty list is a valid request — most test merchants have no wallet connectors configured. Ignored unless `sdk_checkout` is enabled. |

### `scenarios` (required)

Array of `{ "name", "merchant_path", "scenario", "weight" }` — see
*How the traffic split works* and *Scenario catalogue* above. Invalid
combinations (e.g. modular `cit_metadata_changed`) fail validation at
startup.

### `thresholds` (optional)

Raw k6 [thresholds](https://grafana.com/docs/k6/latest/using-k6/thresholds/).
Every metric name below can be referenced, including the per-scenario ones:

```json
"thresholds": {
  "payment_confirm_ms_nomod_guest": ["p(90)<500"],
  "scenario_failure_mod_cit_off": ["count==0"]
}
```

Without thresholds the run always exits 0.

## Finding maximum RPS

Use ramp mode: the run steps the **total** RPS up level by level (your weight
split is applied at every level), and the summary prints confirm p50/p90/p99,
achieved TPS, and success/failure counts **per phase**, so you can see
exactly where the system saturates.

This is the [*breakpoint testing*](https://grafana.com/docs/k6/latest/testing-guides/test-types/breakpoint-testing/)
pattern — also known as capacity, point-load, or limit testing — applied in a
stepped ("staircase") variant: the k6 docs' canonical shape is one continuous
`ramping-arrival-rate` ramp with no plateaus, while this script holds each
level so every step gets steady-state percentiles and failure counts. You
learn *how* the system degrades at each level, not just *where* it breaks.
A run is a breakpoint test when `target_rps` is set unrealistically high and
the goal is to find the failure points (latency degradation → timeouts →
errors → collapse), not merely to survive an expected load. As in the k6
docs, run it only after the other test types pass, and repeat after each
tuning exercise.

To stop the run automatically once the system starts failing instead of
letting it complete all phases, use k6's `abortOnFail` threshold form (the
`thresholds` config passes through verbatim, so objects work):

```json
"thresholds": {
  "payment_confirm_ms_mod_cit_off": [{ "threshold": "p(99)<1000", "abortOnFail": true }],
  "scenario_failure_mod_cit_off": [{ "threshold": "count==0", "abortOnFail": true }]
}
```

Threshold expressions pass while **true** — write them as the *healthy*
condition (`p(99)<1000`, `count==0`, `count<100`). A breach flips the
expression to false, failing the run (exit code 99 when combined with
`abortOnFail`); without `abortOnFail` the run completes and exits non-zero.

```json
"load": {
  "phases": { "starting_rps": 5, "target_rps": 60, "step_rps": 5, "hold_seconds": 30, "idle_seconds": 10 },
  "request_timeout_ms": 30000
}
```

Recipe:

1. Pick the mix that matters — e.g. a single representative entry at
   `weight: 100`, or your expected production mix.
2. Hold each level long enough to reach steady state (`hold_seconds` ≥ 30);
   `idle_seconds` lets queues drain and makes the phases cleanly separated.
3. Read the per-phase table: the **maximum sustainable RPS** is the last
   phase where confirm p99 still meets your SLO with zero (or near-zero)
   failures. A sudden latency/failure jump after near-linear drift is the
   saturation knee; the phase before it is your safe capacity. The table's
   `achieved tps` column (`scenario_success_X_pN` count / `hold_seconds`) is
   what actually completed, so read it alongside `target rps` — at the knee
   phase the two diverge: `target rps` keeps climbing as configured while
   `achieved tps` flattens or drops, since failed/timed-out confirms don't
   count as successful transactions. **Peak RPS is the `target rps` of the
   last healthy phase; peak TPS is that phase's `achieved tps`** — report
   both, since they usually aren't equal even at capacity (some failure
   margin is normal).
4. Guard against a lying client (the `global` row of the summary reports
   both signals):
   - `dropped_iterations` must be 0; otherwise the run never actually
     reached the reported rate — raise the VU multipliers and rerun.
   - Watch the load-generator machine's own CPU.
5. Phases are not perfectly separated at the tail: iterations still in
   flight when a phase ends finish during the next phase (graceful stop).
   Per-phase metrics stay attributed to the phase that *started* each
   iteration, but the extra load bleeds into the next phase — set
   `idle_seconds` at or above your worst-case iteration duration if you
   want clean phase boundaries.
6. Remember the units: the phases count **iterations** (payment flows), and
   each flow fans out to several server-side HTTP requests per iteration
   (≈2 for a `non_modular` guest, ≈6 for a modular CIT flow). Server-side
   HTTP RPS ≈ phase RPS × steps-per-iteration.
7. Correlate with server-side signals (CPU, DB, downstream latency in
   Grafana) to understand *what* saturated; then zoom in with a second,
   narrower ramp around the knee and repeat runs to confirm stability.

Treat an early low-rate phase as warm-up if the numbers look off (cold
pools, JIT-like effects).

### Choosing the SLO to compare against

An SLO is chosen, not discovered. Get numbers from, in order of authority:
contractual SLAs minus buffer; business tolerance; and dependency budgets
(set the target on internal latency via `x-hs-latency`, excluding connector
time, so connector slowness is not counted against the app). If none of
those exist, measure current behavior at expected peak load and set the
target near it (e.g. current p99 × ~1.3 headroom), then ratchet tighter.
This repo's legacy k6 CI used informal anchors of p90 < 500ms for payment
flows and p90 < 15ms for `/health`.

Encode the chosen SLO in `thresholds` so breaches fail the run, e.g.:

```json
"thresholds": {
  "payment_confirm_ms_mod_cit_off": ["p(90)<500", "p(99)<1000"],
  "scenario_failure_mod_cit_off": ["count==0"]
}
```

The highest ramp phase where these hold (with `dropped_iterations` ≈ 0) is
the maximum RPS **at that SLO**.

## Metrics produced

Per scenario entry `X` (`X` = the entry's `name`):

- `<step>_ms_X` — one Trend per executed step (e.g. `payment_create_ms_X`,
  `pm_session_confirm_ms_X`, `payment_confirm_ms_X`; for `sdk_checkout` also
  `payment_method_list_ms_X`, `session_ms_X`, `eligibility_ms_X`), aggregated
  across all phases in ramp mode.
- `total_flow_ms_X` — wall-clock duration of the whole iteration.
- `scenario_success_X` / `scenario_failure_X` — counters; failures carry a
  `reason` tag (visible with `--out json` or other real-time outputs).
- In ramp mode, additionally per phase `N`: `payment_confirm_ms_X_pN` and
  `scenario_success_X_pN` / `scenario_failure_X_pN`. Every executor request
  also carries `traffic_scenario` and `phase` (e.g. `phase_2_20_rps`) tags
  for raw-output analysis.

Across all entries: `payment_confirm_latency_ms`, plus k6's built-ins
(`http_req_duration`, `iterations`, `dropped_iterations`, ...).

The end-of-run output prints a `global` row — total iterations,
**peak iteration rate**, `dropped_iterations` (with a warning when non-zero,
meaning the client could not sustain the requested rate), HTTP request
count and HTTP failure rate — followed by a table per entry: target RPS,
achieved TPS, confirm p50/p90/p99, total-flow p50/p90 (ramp mode omits
total-flow — it's aggregated across phases so isn't phase-comparable), and
success/failure counts.

`peak_iteration_rate` is the highest iteration rate (every iteration,
success or failure — same thing k6's own "Iteration Rate" web-dashboard
tile measures) sustained across the run: in ramp mode, the max over phases
of that phase's total iterations across every scenario ÷ its
`hold_seconds`; in flat mode, the single sustained rate for the whole run.
It's derived after the run completes from the final aggregate counts (same
as `achieved tps` below), not from a live per-second trace, so — like
everything else on this page — it can only appear in this stdout table and
`SUMMARY_OUTPUT` (as the `peak_iteration_rate` gauge), never inside
`timeseries.html`: the web dashboard is k6's own separate output plugin,
with a fixed panel set this script has no way to add to (see *Visualizing
throughput/latency over time*).

`target rps` is the offered/requested iteration rate (`total_rps × weight /
100`, or the phase's rate in ramp mode); `achieved tps` is
`scenario_success_X[_pN] count / duration_seconds` — the actual completed
transactions per second. They diverge whenever confirms fail, time out, or
iterations get dropped, which is exactly the signal that matters at
saturation. Note that this custom output replaces k6's default end-of-test
summary; `SUMMARY_OUTPUT=/path/summary.json` additionally exports the full
summary (including all percentile stats) as JSON — see *Quick start*.

## Collecting everything into one output directory

`scenario-mix.js`'s own output (`SUMMARY_OUTPUT`, if set) defaults to the
`output/` directory checked into this repo alongside the script, rather than
wherever you ran `k6 run` from:

```bash
SUMMARY_OUTPUT=summary.json SCENARIO_MIX_CONFIG=config.json k6 run scenario-mix.js
# -> output/summary.json
```

Override where it goes with `OUTPUT_DIR`:

```bash
mkdir -p out/run1   # see note below — non-default dirs must exist before the run
OUTPUT_DIR=out/run1 SUMMARY_OUTPUT=summary.json SCENARIO_MIX_CONFIG=config.json \
  k6 run scenario-mix.js
# -> out/run1/summary.json
```

`SUMMARY_OUTPUT`, if you set it, is treated as a filename *within*
`OUTPUT_DIR`, not a full path — `OUTPUT_DIR=out/run1
SUMMARY_OUTPUT=confirm-summary.json` writes `out/run1/confirm-summary.json`.

**A non-default `OUTPUT_DIR` must already exist** (`output/` itself is
already there, so this only matters when you override it). k6 VU/init code
has no filesystem write access — the actual file write for
`handleSummary`'s return value happens in k6's Go runtime afterward, and it
fails outright (`could not open '.../summary.json': ... no such file or
directory`) if the directory is missing. There's nothing `scenario-mix.js`
can do about this from inside the script; always `mkdir -p "$OUTPUT_DIR"`
first when using a custom one.

The same constraint applies to outputs that aren't controlled by the script
at all — `--console-output` (failure log, below) and
`--out web-dashboard=export=...` (time-series export, below) are k6 CLI
flags, not something `handleSummary` can redirect, so they don't
automatically follow `OUTPUT_DIR`. Point them at `output/` explicitly (no
`mkdir` needed, since it already exists) to get everything from one run in
one place:

```bash
SUMMARY_OUTPUT=summary.json SCENARIO_MIX_CONFIG=config.json \
  k6 run -q --console-output=output/failures.log \
  --out 'web-dashboard=export=output/timeseries.html' \
  scenario-mix.js
# -> output/{summary.json,failures.log,timeseries.html}
```

Or point them (and `OUTPUT_DIR`) at a different, freshly created directory
the same way as above if you want each run kept separate rather than
overwriting `output/` every time.

## Visualizing throughput/latency over time

The stdout table and `SUMMARY_OUTPUT` above are an **end-of-run** view —
aggregated percentiles for the whole run or, in ramp mode, per phase. To
see how throughput and latency *evolved* second by second — and for a
shareable HTML result of the run in general, since there's no other HTML
report — use k6's built-in web dashboard output: a live, in-browser,
auto-updating set of charts (RPS, latency percentiles, VUs, error rate,
plus a panel per custom metric, including this script's per-scenario/
per-step Trends) that can also be exported to a static file:

```bash
# Live view while the test runs — open http://127.0.0.1:5665 in a browser:
k6 run -q --out web-dashboard -e SCENARIO_MIX_CONFIG=config.json scenario-mix.js

# Save it as a static HTML file you can keep/share afterward — this is the
# recommended way to run scenario-mix.js in general, not just for ramps:
k6 run -q --out 'web-dashboard=export=timeseries.html&period=2s' \
  -e SCENARIO_MIX_CONFIG=config.json scenario-mix.js
```

`period` sets the chart's time-bucket size (default `10s`); shorten it for
short runs, lengthen it for long soaks. This is independent of
`handleSummary`/`SUMMARY_OUTPUT` — the outputs don't conflict, so a single
run can produce both a live/exported time-series view *and* the end-of-run
summary. Note: very short runs (well under a minute) are skipped — `k6`
logs `"the test run was short, report generation was skipped"` and no
export file is written — this is meant for real runs, not smoke tests.

If you already have Prometheus/Grafana running and would rather push metrics
there (e.g. to compare runs over time, or correlate with Router's own
metrics), k6 also supports `--out experimental-prometheus-rw=<remote-write-url>`
directly — no extra provisioning needed on the k6 side. (The
`loadtest/grafana/dashboards/k6-load-testing-results_rev3.json` dashboard in
this repo was built for the older docker-compose `loadtest/loadtest.sh`
flow, not for `scenario-mix.js` — see *Relationship to `loadtest/runner`*
below.)

## Logging failures to a file

The `scenario_failure_X[_pN]` counters (and the summary's `failure` column)
tell you *how many* iterations failed and *why* in aggregate (the `reason`
tag), but not the detail behind any individual failure. For that,
`scenario-mix.js` logs one JSON record per failed iteration via
`console.error` — scenario name, merchant path, phase (ramp mode), VU,
iteration number, the same `reason` string used in the counters, and, when
the failure came from an HTTP call, that response's status, URL, connector/
network error, and body (truncated to 500 characters).

k6 VU code can't write files directly — `open()` is read-only and only
usable during init — so getting these into a file means redirecting k6's own
console output, via `--console-output` or `K6_CONSOLE_OUTPUT`:

```bash
k6 run --console-output=failures.log -e SCENARIO_MIX_CONFIG=config.json scenario-mix.js
# or
K6_CONSOLE_OUTPUT=failures.log SCENARIO_MIX_CONFIG=config.json k6 run scenario-mix.js
```

Only failures go to `failures.log` — the end-of-run table still prints to
the terminal as usual. Each line looks like:

```
time="2026-09-08T13:29:10+05:30" level=error msg="{\"time\":\"2026-09-08T07:59:10.867Z\",\"scenario\":\"mod_cit_off\",\"merchant_path\":\"modular\",\"scenario_type\":\"cit_off_session\",\"phase\":2,\"vu\":6,\"iteration\":14,\"reason\":\"payment_confirm_400_failed\",\"status\":400,\"url\":\"http://127.0.0.1:8080/payments/pay_.../confirm\",\"body\":\"{...}\"}"
```

k6 wraps each record in its own log line (`time=... level=error msg="..."`)
with the JSON escaped inside `msg`; pull it back out with `jq`, e.g.
`grep -o 'msg="{.*}"' failures.log | sed 's/^msg="//; s/"$//' | sed 's/\\"/"/g' | jq .`,
or just read it as-is — the escaped JSON is still human-readable inline.
Without `--console-output`/`K6_CONSOLE_OUTPUT` set, these lines print to the
terminal interleaved with k6's own progress output instead.

## Relationship to `loadtest/runner`

| | `runner` | `scenario-mix` |
| --- | --- | --- |
| Purpose | Regression-grade per-scenario benchmarks with fixture isolation | Quick mixed-traffic characterization of a running stack |
| Iterations | Fixture stage pre-creates payments; measured stage confirms only | Fully self-contained; create + confirm inline |
| Load shape | Ramps (start → target with steps and idle gaps) | Flat RPS per entry, or stepped ramp with per-phase metrics |
| Environment | Deploys/configures services, merchants, Superposition | Nothing provisioned; existing credentials only |

Both honor the same scenario definitions; if you add a flow to
`runner/lib/scenarios.js`, mirror it in the `SCENARIOS` table at the top of
`scenario-mix.js`.

## Development checks (no services needed)

```bash
# Config validation + options build without sending traffic.
# NOTE: `k6 inspect` only accepts env vars via -e, not the process environment.
k6 inspect -e SCENARIO_MIX_CONFIG=config.example.json scenario-mix.js
```
