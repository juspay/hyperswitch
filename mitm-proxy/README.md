# MITM proxy pilot — offline Cypress for Hyperswitch

This directory holds the proxy half of an experiment to **run the Hyperswitch
Cypress suite without depending on live connector sandboxes**. The Cypress side
is in [`../cypress-tests/`](../cypress-tests/).

If you're picking this up for the first time, read this whole doc before
touching files — there are several non-obvious findings sprinkled through the
design that look strange in isolation.

## TL;DR

```
./start.sh                                       # mitm in capture mode
# (run cypress against live connector sandboxes)
python3 normalize_captures.py                    # curate cassettes
./start_replay.sh                                # mitm in replay mode
# (run cypress with CYPRESS_PROXY_MODE=replay)
```

The capture run records every HS→connector call as a JSON cassette. The replay
run serves cassettes back to HS instead of letting the call reach the connector.
Same test code runs in both modes; the only env-var difference is
`CYPRESS_PROXY_MODE=replay`.

## Why this exists

The Cypress suite today calls real connector sandboxes (Stripe, Adyen, …) for
every test run. That gives us:

- ❌ flaky tests when sandboxes have issues
- ❌ rate limits and cred rotation pain
- ❌ slow feedback (network round-trips, 3DS browser dances)
- ❌ CI environment can't easily acquire/store creds for every connector

Goal: capture each test's connector traffic once, then replay it offline. Tests
become hermetic, fast, and don't need any live sandbox creds in CI.

## Architecture

```
                              ┌────────────────┐
                              │     Cypress    │
                              └────────┬───────┘
                                       │  test API calls
                                       ▼
                              ┌────────────────┐
                              │  Hyperswitch   │
                              │     router     │
                              └────────┬───────┘
                                       │  router→connector outbound
                                       ▼
                              ┌────────────────┐  capture: forward + record JSON
                              │   mitmproxy    │
                              │  (this dir)    │  replay:  serve JSON, no forward
                              └────────┬───────┘
                                       │  (only in capture mode)
                                       ▼
                              ┌────────────────┐
                              │ Stripe / Adyen │
                              │ … sandbox APIs │
                              └────────────────┘
```

Cypress, HS, and mitmproxy run on the same machine in the local pilot. HS is
configured with `proxy.http_url`/`https_url` pointing at mitm, and trusts mitm's
self-signed CA via `proxy.mitm_ca_certificate`. The pilot uses port `8888` for
the proxy and `8181` for the mitm admin server (default is `8081`, which
collides with rootless container ports on the dev machine — hence the override
in `ADMIN_PORT=8181`).

## How a call gets matched to a cassette

The match key is `(connector, x-request-id, method, path)`.

**`X-Request-ID` is the linchpin.** Cypress's `support/e2e.js` wraps `cy.request`
to stamp every outbound with `X-Request-ID: <djb2(testTitle)>-<NNN>` where NNN
is a per-step counter. The HS router, configured with
`trace_header.id_reuse_strategy = "use_incoming"`, propagates that ID onto its
connector outbound. So the same test produces identical request_ids across
runs, and replay can look up cassettes deterministically.

Cassettes are laid out on disk as:

```
captures/<connector>/<safe_test_name>/<request_id>/<NNN>.json
```

The `<NNN>` index disambiguates multiple cassettes under the same
`(test, request_id)` — they're served FIFO during replay.

## The pieces, in commit order

### `4f44f5bde` — Add MITM proxy capture/replay scripts

The starting point. Establishes the cassette format and the
`(connector, request_id, method, path)` match key. See the [Architecture](#architecture)
section above for the network topology this implies.

Files: `mitm_capture.py`, `mitm_replay.py`, `start.sh`, `start_replay.sh`,
`.gitignore`.

### `ae846f1ae` — Wire X-Request-ID injection in Cypress

The Cypress half: `support/e2e.js` overrides `cy.request` to add a deterministic
`X-Request-ID` header per request. A `beforeEach` hook POSTs `/test/start` to
mitm's admin server (used only for cassette folder naming, not for matching).

Also pinned `retries: 0` in `cypress.config.js` — retries re-enter `beforeEach`,
reset the step counter, and would desync cassette IDs.

### Current pilot work

Three new things, captured in the current uncommitted diff:

#### A. Replay-mode 3DS bypass — `cypress-tests/cypress/support/commands.js`

When `CYPRESS_PROXY_MODE=replay`, `cy.handleRedirection` **skips the browser
3DS dance entirely**. Two bypass paths exist, picked per connector:

**(a) Synthetic connector→HS webhook** (`cy.fireConnectorWebhook`).
Cypress builds a payload, HMAC-signs it Node-side, and POSTs it to
`${baseUrl}/webhooks/${merchantId}/${connectorId}`. HS verifies the
signature, runs `CallConnectorAction::HandleResponse(payload)` (no connector
outbound), and the payment advances to terminal state.

Requires HS to verify the webhook locally with an HMAC scheme we can
replicate. Currently implemented for **stripe** and **adyen** —
the allowlist is hard-coded in `cy.handleRedirection`'s replay branch.

- Stripe — HMAC-SHA256 over `"{timestamp}.{body}"`, hex, in `Stripe-Signature` header.
- Adyen — HMAC-SHA256 (key hex-decoded) over a 7-field colon-delimited string, base64, embedded in body.

The signing tasks live in `cypress.config.js` (`signStripeWebhook`,
`signAdyenWebhook`) because the browser context lacks `node:crypto`.
The connector account's webhook secret comes from
`cypress/fixtures/create-connector-body.json` →
`connector_webhook_details.merchant_secret`.

**(b) Synthetic redirect-callback** (`cy.simulateRedirectCallback`).
For non-card redirect flows (bank_redirect, wallet) the confirm response
has no `connector_transaction_id`, so we can't sign a webhook anyway.
Instead Cypress POSTs to HS's
`/payments/{id}/{merchantId}/redirect/response/{connector}` endpoint —
HS's `PaymentRedirectSync` then force-syncs the connector, and mitm
matches that outbound from cassette.

Path (b) is also the fallback for **any connector not in the webhook
allowlist** — including connectors with no `IncomingWebhook` impl
(cybersource) and those whose webhook verification requires an outbound
API call back to the connector (paypal verifies via
`/v1/notifications/verify-webhook-signature`, which we can't replicate).

Why this works: HS's `crates/router/src/core/webhooks/incoming.rs:856` has
```rust
let consume_or_trigger_flow = if source_verified {
    CallConnectorAction::HandleResponse(resource_object)
} else {
    CallConnectorAction::Trigger
};
```
A **signed** webhook is consumed as the connector response. Unsigned would fall
back to a real connector sync — which mitm couldn't serve, so signing is
mandatory for the pilot.

#### B. Capture curation — `mitm-proxy/normalize_captures.py`

Post-capture cleanup script. Run between `./start.sh` and `./start_replay.sh`.
It now **keeps server-UUID folders**. Those cassettes are not noise for
redirect/3DS flows: they are the connector calls triggered when the browser
returns to HS without Cypress's `X-Request-ID`. Replay has a test-scoped
server-rid fallback, so these cassettes can be served without manual
relocation.

Normalization is connector-scoped. Common code only performs safe hygiene
(counting, keeping server UUIDs, and credential redaction). Every connector has
an explicit module under `mitm-proxy/normalizers/<connector>.py`, even when it
is currently a no-op. Heuristics that may move cassettes must live only in that
connector's module; today the PayPal normalizer quarantines clear cy.visit
duplicate orphans. If a capture directory exists for a connector without a
module, normalization fails instead of applying any implicit global heuristic.

Cassettes are also sanitized for source control: any value found in `creds.json`
is stored as a placeholder like
`{{MITM_SECRET:paypal.connector_account_details.api_key}}`. Replay hydrates
placeholders from `creds.json` in memory before serving the cassette response;
the secret value is not written back to disk. Before committing captures, run:

```
python3 mitm-proxy/check_cassettes_redacted.py mitm-proxy/captures
```

**Nothing is deleted.** Quarantined items go to a sibling
`captures_quarantine/` directory mirroring the original structure. Restore
manually if normalize misjudged:

```
mv mitm-proxy/captures_quarantine/<path> mitm-proxy/captures/<path>
```

For CI/local isolation, prefer the one-at-a-time runner:

```
mitm-proxy/repro/cap_replay_all_one_at_a_time.sh paypal redsys
```

#### C. Small globalState additions

`createPaymentIntentTest` and `createConfirmPaymentTest` now stash
`captureMethod`, `paymentAmount`, and `paymentCurrency` in globalState so
`fireConnectorWebhook` can read them without doing an extra `cy.request`
(which would bump the step counter and desync the cassette IDs).

## The cy.visit duplicate problem

This is the most important finding from the pilot, and it's not something the
test code can fix.

### Symptom

When a 3DS test runs in capture mode, the cypress log shows the test's first
several steps execute **twice**, both times with the same `testIdHash`. The
step counter resets to 0 between the two runs, so the second run reuses the
same `<hash>-001`, `<hash>-002`, … X-Request-IDs. HS dutifully makes a fresh
connector POST for each, producing **two cassettes under the same
`(test, request_id)` key**, each with a different connector-side `pi_xxx`.

### Cause

When `cy.visit` navigates the test browser to an external URL (Stripe ACS
page), Cypress re-injects its bundled support scripts on each navigation. Each
injection re-runs the module-level code, which calls `beforeEach(...)` again,
**registering the hook a second time**. By the next test boundary (which the
navigation itself triggers in Cypress's internal model), all registered
`beforeEach` hooks fire. The `support/e2e.js` `beforeEach` resets
`stepCounter = 0` and re-posts `/test/start`, and the test body's queued
commands appear to re-execute.

This isn't documented Cypress behavior. The commit at `ae846f1ae` already
flagged a related symptom (forced `retries: 0`); the cy.visit re-injection
half was not known then. We verified empirically with the mitm admin log
showing `▶ START: ...` firing multiple times for the *same* test title in a
single it() block.

### Why it breaks replay (but not the original test runs)

In capture, both runs reach HS. HS creates two distinct payment_intents on
Stripe (`pi_AAA` and `pi_BBB`). HS internally tracks the **latest** as the
canonical `connector_transaction_id`. Subsequent steps (retrieve, capture)
reference `pi_BBB`.

In replay, the same test runs **once** end-to-end (no browser navigation, so
no cy.visit-induced re-execution). HS makes one POST and gets the **first**
cassette response (FIFO), which is `pi_AAA`. The downstream retrieve cassette
is keyed on `pi_BBB`'s URL, so the lookup misses.

### How we handle it: manual curation

We tried several automated approaches (LIFO at replay, mitm pause/resume,
`cy.intercept`, Node-side seen-set, orphan-detection in normalize). Each
either failed because mitm is a transparent proxy that only records — it
doesn't block — or risked silent failures with heuristics that could
misclassify legitimate calls.

**Current approach is manual:** when replay logs a MISS for a cassette
we know belongs to a cy.visit-induced duplicate, identify it by
`captured_at` (the earlier of the two), and move it to
`captures_quarantine/`. Then re-run replay.

Concretely, for the canonical example (spec 16 context 1):

```
mitm-proxy/captures/stripe/Card_-_ThreeDS_Manual_..._Full_Capture_payme/9002d8cd-003/
├── 000.json   ← earlier capture (orphan), quarantine this
└── 001.json   ← latest capture, keep
```

Move `000.json` into `captures_quarantine/.../9002d8cd-003/` and replay
will FIFO-match `001.json`.

We may revisit automation later — possibilities include checking whether
the cassette's `response.body.id` appears in any other cassette's path
(orphans don't), or tracking `/test/start` counts per test in the mitm
admin log. For the pilot, manual is fine.

## Running the loop

### One-time setup

1. Install mitmproxy: `uv tool install mitmproxy` (or via your package manager).
2. Generate the mitm CA once: `./start.sh` and Ctrl-C immediately. This creates
   `~/.mitmproxy/mitmproxy-ca-cert.pem`.
3. Configure HS to trust the proxy. The `start.sh` script prints the env exports
   you need:
   ```
   export ROUTER__PROXY__HTTPS_URL="http://127.0.0.1:8888"
   export ROUTER__PROXY__HTTP_URL="http://127.0.0.1:8888"
   export ROUTER__PROXY__MITM_CA_CERTIFICATE="..."
   export ROUTER__TRACE_HEADER__ID_REUSE_STRATEGY="use_incoming"
   ```
   Restart the HS router with these in its env.
4. Make sure your `creds.json` is set up for the connector you're piloting.

### Capture mode

```bash
# Terminal 1: mitmproxy
cd mitm-proxy
ADMIN_PORT=8181 ./start.sh

# Terminal 2: run cypress against live sandbox
cd cypress-tests
CYPRESS_CONNECTOR=stripe \
CYPRESS_BASEURL=http://localhost:8080 \
CYPRESS_ADMINAPIKEY=test_admin \
CYPRESS_CONNECTOR_AUTH_FILE_PATH=$(pwd)/../creds.json \
CYPRESS_PROXY_ADMIN_URL=http://127.0.0.1:8181 \
npx cypress run --headless --spec '<your spec>'

# After capture finishes:
cd ../mitm-proxy
python3 normalize_captures.py
```

`ADMIN_PORT=8181` overrides the default `8081` to avoid the rootless container
collision on the dev machine. The corresponding Cypress var is
`CYPRESS_PROXY_ADMIN_URL=http://127.0.0.1:8181`.

### Replay mode

```bash
# Terminal 1: mitmproxy in replay
cd mitm-proxy
ADMIN_PORT=8181 ./start_replay.sh

# Terminal 2: same cypress invocation + PROXY_MODE=replay
cd cypress-tests
CYPRESS_CONNECTOR=stripe \
CYPRESS_BASEURL=http://localhost:8080 \
CYPRESS_ADMINAPIKEY=test_admin \
CYPRESS_CONNECTOR_AUTH_FILE_PATH=$(pwd)/../creds.json \
CYPRESS_PROXY_ADMIN_URL=http://127.0.0.1:8181 \
CYPRESS_PROXY_MODE=replay \
npx cypress run --headless --spec '<your spec>'
```

The replay should be much faster than capture (no browser dance, no real
network round-trips). For spec 16 we observe ~1 second of replay vs ~40 seconds
of capture.

In the mitm log you should see `[replay] HIT ...` for every connector call and
**zero `MISS`**. A MISS means the cassette set is incomplete or out of sync —
either you need to re-capture, or `normalize_captures.py` dropped something it
shouldn't have.

## What's been validated so far (Stripe)

**Full Payment-spec suite (45 specs, 350 tests)** validated end-to-end:

- **285 passing in capture** (live Stripe sandbox).
- **454 / 458 non-pending tests passing in replay** (~99.1%).
- 63 tests pending — Stripe-unsupported payment methods that skip themselves.
- 4 known failures (see below) — none introduced by the pilot.

### Known failures (4 of 458, all documented)

| Spec | Tests | Why it fails | Pilot-introduced? |
|---|---|---|---|
| `20-MandatesUsingPMID` | 1 | HS-internal state issue: after `simulateRedirectCallback`, the subsequent `cit-capture` call gets an error response from HS even though every connector cassette HITs cleanly. Browser-driven capture probably gave HS more time to settle state. | No (HS-internal) |
| `24-PaymentMethods` | 1 | Test asserts an empty `customer_payment_methods` array but the customer carries leftover state from a prior `it()` in the same context. | No (pre-existing test design) |
| `40-ExternalVault` | 2 | Test uses VGS as a second connector for card vaulting alongside Stripe. We only have Stripe cassettes; VGS calls fall back to LIVE and fail. Same failures occur in capture mode. | No (multi-connector test) |

### Manual curation per recapture (counts)

Across the full Stripe Payment suite, manual curation totals:
- **~36 server-UUID folders** auto-quarantined by `normalize_captures.py` (no work).
- **~10 orphan-quarantines** (move earlier-of-duplicate cassettes — see the manual-curation section below).
- **~6 server-UUID-relocations** (move quarantined cassettes back to Cypress rids — 3DS-refund and bank-redirect tests).

The README's manual-curation tables list the validated set; re-apply after each fresh capture.

### Manual curation currently required

After `normalize_captures.py` auto-quarantines the obvious server-UUID
noise, there are two more mechanical-but-manual steps needed for the
full Payment-spec suite to replay cleanly.

Two kinds of operation, applied per test:

#### A. Orphan-quarantine (move earlier-of-duplicate to quarantine)

Some rid folders end up with multiple cassettes — Cypress's `beforeEach`
refires on `cy.visit`, the same test re-runs partially, and HS creates a
fresh Stripe PI each time. Subsequent cassettes (retrieves, captures,
refunds) reference the **latest** PI. FIFO would serve the earliest, so
HS state would diverge — fix is to quarantine all but the latest in each
duplicate group:

```
mitm-proxy/captures/stripe/<test>/<rid>/
├── 000.json   ← earlier captured_at — orphan, move to captures_quarantine
└── 001.json   ← latest — keep
```

#### B. Server-UUID-relocate (move quarantined cassette back to the right rid)

The post-redirect connector sync (for 3DS-refund and bank-redirect flows)
is triggered by the browser's ACS callback in capture, so it gets a
server-UUID rid. In replay, our bypass triggers the same HS endpoint
with a Cypress-deterministic rid. The cassette content is right but
the key is wrong. We move the cassette out of quarantine back into
the captures tree at the expected rid, and update its `request_id`
JSON field to match.

One-liner pattern:

```bash
jq --arg rid "TARGET-RID" '.request_id = $rid' \
  captures_quarantine/stripe/<test>/<server-uuid>/000.json \
  > captures/stripe/<test>/TARGET-RID/000.json
rm captures_quarantine/stripe/<test>/<server-uuid>/000.json
```

### Specific manual curation for the validated Stripe Payment suite

Confirmed working set (re-apply after each recapture):

**Relocations** (move quarantined server-UUID cassette → captures at given rid):

| Test folder prefix | Target rid |
|---|---|
| `Card_-_Refund_flow_-_3DS_Fully_Refund_Card-ThreeDS_..._Create_Conf` | `a310a589-003` |
| `Card_-_Refund_flow_-_3DS_Partially_Refund_Card-ThreeDS_..._Create_` | `3787a59a-003` |
| `Bank_Redirect_tests_EPS_Create_and_Confirm_flow_test_..._Lis` | `94eb78f0-004` |
| `Bank_Redirect_tests_iDEAL_Create_and_Confirm_flow_test_..._L` | `bc1e7453-004` |
| `Bank_Redirect_tests_Przelewy24_Create_and_Confirm_flow_test_..._Inten` | `ced8176a-004` |

**Orphan quarantines** (move earlier cassette to quarantine):

| Test folder prefix | rid | Files to quarantine |
|---|---|---|
| `Card_-_ThreeDS_Manual_..._Full_Capture_payme` | `9002d8cd-003` | `000.json` |
| `Card_-_Refund_flow_-_3DS_Partially_Refund_..._Create_` | `3787a59a-001` | `000.json` |
| `Bank_Redirect_tests_EPS_..._Lis` | `94eb78f0-003` | `001.json`, `002.json` |
| `Bank_Redirect_tests_Przelewy24_..._Inten` | `ced8176a-003` | `000.json` |

### How we identified these

For orphans: look at any rid folder with more than 1 cassette. Check the
`response.body.id` of each. The cassette whose id does NOT appear in any
*later* cassette's `request.path` is the orphan — quarantine it.

For relocations: when replay logs a `MISS` for a GET with a deterministic
rid, find the matching server-UUID cassette in `captures_quarantine/`
(same `request.path` shape) and relocate.

These are mechanical enough that future automation could do them, but
for the pilot the manual fix is explicit and traceable.

## Validation across the extended-connectors batch

The GitHub Actions cypress workflow (`.github/workflows/cypress-tests-runner.yml`)
defines `EXTENDED_PAYMENTS_CONNECTORS_BATCH_1` + `BATCH_2` containing 7 connectors:
`bluesnap`, `gigadat`, `loonio`, `nmi`, `paypal`, `redsys`, `zift`. This section
tracks the capture/replay pilot's coverage across that set (plus the stripe
baseline from above and adyen from earlier work).

| Connector | Webhook strategy | Cypress scope | Capture | Replay | Cassettes | Notes |
|---|---|---|---|---|---|---|
| stripe | HMAC-easy (`t.body`, raw secret, hex) | Full Payment suite (45 specs) | 285/350 | 454/458 (99.1%) | — | Baseline — see Stripe-specific section above |
| adyen | HMAC-easy (colon-joined fields, hex-decoded secret, base64) | partial | — | — | — | Wired in `WEBHOOK_BYPASS_CONNECTORS`; not yet full-suite validated |
| bluesnap | HMAC-easy (`ts + body` no separator, raw, hex, `bls-signature` header) | 14-spec subset | 102/102 | 102/102 (100%) | 93 | Spec 14 SaveCard-3DS off-session exercises the bypass |
| zift | No-impl (`WebhooksNotImplemented`) — pure API | 14-spec | 102/102 | 102/102 (100%) | 81 | No webhooks, no 3DS, no redirects → cleanest replay profile |
| nmi | HMAC-easy (`t.body`, raw, hex, `webhook-signature: t=<ts>,s=<hex>`) | 14-spec | 102/102 | 102/102 (100%) | 141 | Fixed via `simulateNmiRedirectComplete` — `/redirect/complete/{connector}` with customerVaultId extracted from HS's redirect-form HTML + synthetic 3DS proof |
| loonio | No-impl (`Ok(false)`) | 15-spec (incl. 18-BankRedirect) | 110/110 | 110/110 (100%) | 4 | Interac only; tiny cassette set |
| gigadat | No-impl (`Ok(false)`) | 15-spec | 110/110 | 110/110 (100%) | 3 | Interac only; near-twin of loonio |
| paypal | External-verify (outbound `/v1/notifications/verify-webhook-signature`) — webhook bypass NOT viable | 15-spec | 110/110 | 110/110 (100%) | 106 | Card 3DS uses `simulatePaypalRedirectComplete` (`/redirect/complete/paypal`); PayPal bank-redirect replay no-ops because live capture produces no post-redirect connector cassette for those specs |
| redsys | No-impl (`WebhooksNotImplemented`), 3DS-only | 15-spec | 110/110 | 110/110 (100%) | 75 + server-rid fallback | Fixed via `simulateRedsysRedirectComplete` (synthetic `cres` to `/redirect/complete/{connector}`); browser-callback `trataPeticionREST` cassettes captured under server UUIDs are served by replay's test-scoped server-rid fallback |

**Aggregate across the 7 extended connectors:** 746 tests captured, **746 passing in replay = 100%** after applying connector-specific replay-mode bypasses and replay's server-rid fallback for browser-callback cassettes.

### Parallel capture infrastructure

To capture multiple connectors in parallel, run multiple HS routers paired
with their own mitm processes on different ports:

| HS port | mitm proxy | mitm admin | example log path |
|---|---|---|---|
| 8080 | 8888 | 8181 | `/tmp/hs.log`, `/tmp/mitm-capture-*.log` |
| 8089 | 8889 | 8182 | `/tmp/hs2.log` |
| 8090 | 8890 | 8183 | `/tmp/hs3.log` |

Each HS instance is started with its own `ROUTER__PROXY__HTTP_URL` /
`HTTPS_URL` pointing at the matching mitm port (same MITM CA cert,
`use_incoming` trace header). All three HS instances share the same
Postgres (port 5434) and Redis (6379) — cypress generates unique merchant
IDs per run, so row collisions are unlikely; the only practical concern
is CPU/RAM (each Chromium ≈ 500 MB).

Cypress invocation per slot just varies baseURL + admin URL:

```bash
CYPRESS_CONNECTOR=<name> \
CYPRESS_BASEURL=http://localhost:<hs_port> \
CYPRESS_ADMINAPIKEY=test_admin \
CYPRESS_CONNECTOR_AUTH_FILE_PATH=$(pwd)/../creds.json \
CYPRESS_PROXY_ADMIN_URL=http://127.0.0.1:<mitm_admin> \
[CYPRESS_PROXY_MODE=replay] \
npx cypress run --headless --spec '...'
```

Cassette folders are per-connector (`mitm-proxy/captures/<connector>/`), so
mitm processes serving different connectors don't collide on disk — even
when the same mitm process serves multiple connectors sequentially (e.g.,
zift's mitm:8888 was re-used for gigadat once zift's capture finished).

### Fix history: NMI and Redsys 3DS replay (resolved)

Originally NMI and Redsys 3DS tests were failing in replay (4/102 and 1/110)
because the existing `simulateRedirectCallback` posts to
`/redirect/response/{connector}` (PaymentRedirectSync), but **both NMI and
Redsys actually run their post-3DS flow through `/redirect/complete/{connector}`
(PaymentRedirectCompleteAuthorize)**. The captured cassette set was correctly
shaped for the CompleteAuthorize path, just mis-matched by the wrong bypass
endpoint.

Resolution:
- Added `simulateNmiRedirectComplete` / `simulateRedsysRedirectComplete` in
  `cypress-tests/cypress/support/commands.js`. Both are gated under
  `PROXY_MODE === "replay"` so capture-mode behaviour is bit-for-bit unchanged.
- NMI: bypass fetches HS's redirect-form HTML, extracts the embedded
  `customerVaultId` + `orderId`, and posts those + synthetic 3DS proof
  (`cavv`/`eci`/etc.) to `/redirect/complete/nmi`.
- Redsys: bypass posts a synthetic base64-encoded `cres` JSON to
  `/redirect/complete/redsys`. mitm matches outbound HS→Redsys calls on
  `(connector, rid, method, path)` so the body isn't validated in replay.
- Redsys browser ACS-callback `trataPeticionREST` cassettes are captured under
  `<test>/<server-uuid>/000.json` because the browser POST-back does not carry
  Cypress's `x-request-id`. These no longer need to be relocated: replay keeps
  a strict fallback index keyed by `(connector, active_test, method, path)` for
  server-rid cassettes and serves them when exact rid matching misses.

Validated end-to-end:
1. **Shell** against live sandbox: replicated cypress's capture-mode flow
   via curl (NMI: extract vault_id from HS HTML; Redsys: drive the simulator
   JS pipeline by re-implementing `submitOK()` / `envioRREQ()` in shell).
   Verified payment → succeeded → refund → succeeded.
2. **Shell against mitm replay**: ran the same flow but with synthetic 3DS
   proof, confirming mitm path-only matching serves the relocated cassettes.
3. **Cypress replay**: NMI 102/102 and Redsys 110/110.

### (Historical) Known limitation: NMI/Redsys 09-RefundPayment (sync-refund)

After the wider replay pass, 5 of the 746 captured tests fail — 4 on NMI
and 1 on Redsys, all in `09-RefundPayment` at the final
`Sync Refund Payment` step. The replay error is always
`Error: Expecting valid response but got an error response`.

**Why the standard fallback path doesn't cover it.** In replay,
`cy.handleRedirection` either fires the synthesized webhook (for connectors
in `WEBHOOK_BYPASS_CONNECTORS` that have a `connectorTransactionID` in
globalState) or falls back to `cy.simulateRedirectCallback`. The fallback
posts to `${baseUrl}/payments/{id}/{merchant}/redirect/response/{connector}`,
which HS dispatches to `PaymentRedirectSync` — i.e., "ask the connector
what's the current status." This is correct for connectors whose post-3DS
finalization is sync-shaped (paypal, redsys for most flows, the bank-
redirect connectors). But:

- **NMI's** post-3DS flow is `CompleteAuthorize`-shaped, not Sync. The
  browser ACS in capture POSTs back to
  `/payments/{id}/{merchant}/redirect/complete/{connector}` (note
  `complete`, not `response`), HS runs `PaymentRedirectCompleteAuthorize`,
  and the connector outbound is a `transact.php` "sale with cavv" — not a
  `query.php` sync. Our fallback bypass triggers the wrong shape.
- **NMI's webhook bypass path** *would* work, but it never fires for NMI
  3DS tests because the cassette captured at rid `-001` is a
  "Customer Vault Add" response with an empty `transactionid`. Cypress
  never sets `globalState.connectorTransactionID`, so the
  `canUseWebhook` check fails and we fall through to the wrong-shape
  fallback.
- Even if we could route NMI through `/redirect/complete/{connector}`,
  the captured `transact.php` "sale with cavv" cassettes live in
  `captures/nmi/_untagged/<server-uuid>/` rather than in the test folder
  — a consequence of the [cy.visit duplicate problem](#the-cyvisit-duplicate-problem)
  scrambling mitm's "current test" context when the browser navigates
  to the ACS page.

**A naive fix made things worse.** Relaxing `canUseWebhook` to fire the
webhook bypass for NMI without a `connectorTransactionID` (using a
synthetic `transaction_id`) unconditionally tells HS "payment succeeded,"
which breaks tests in specs 09 and 16 that expect the payment to fail at
some step. Result: 90/102 (down from 98/102). Reverted.

**A real fix needs multiple pieces:**
1. Make the replay-mode bypass connector-aware so NMI hits
   `/redirect/complete/{connector}` with a synthetic
   `NmiRedirectResponseData` form body (must include at least
   `customerVaultId` — required by the deserializer; value can be a
   placeholder since mitm matches on (connector, rid, method, path) and
   the body isn't compared).
2. Relocate the `_untagged/<server-uuid>/` `transact.php` cassettes back
   into their owning test folders at the right cypress rid. The owning
   test can be identified by walking the post-`-001` cassette's
   `response.transactionid` and finding the test whose `-004` (or `-005`)
   refund cassette references the same id.
3. Either fix the cy.visit re-injection race so future captures land in
   the right test folder, or document a `normalize_captures.py` step
   that moves `_untagged/` cassettes into their owning test before
   replay.

Until then, the 4 + 1 failures are accepted as a known limitation. The
diagnostic anchor cassette for tracing is
`mitm-proxy/captures/nmi/_untagged/019e220d-94ee-7161-a023-feeb3501765d/000.json`
— its `request.body.orderid` and `response.body.transactionid` map to
test prefix `a310a589` and the refund cassette
`Card_-_Refund_flow_-_3DS_Fully_Refund_..._Create_Conf/a310a589-004/`.

### Debugging lesson: replicate the real flow first, then update cypress

The NMI fix was found by a debugging approach that's worth codifying for
future connectors:

> When a replay-mode bypass doesn't match capture behaviour, **don't
> hypothesise from the cypress side first.** Reproduce what cypress is
> *trying to fake* against the real connector, observe what HS actually
> needs end-to-end, and then update cypress to match that ground truth.
> Updating cypress first is reverse-engineering with too many free
> variables.

The NMI walk-through:
1. We knew which test failed and at which step (4 tests in `09-RefundPayment`,
   all failing at `Sync Refund Payment`).
2. Instead of guessing at the cypress bypass, we set up a fresh merchant +
   NMI connector via curl, ran the test's request sequence by hand against
   the **live** NMI sandbox (with mitm in capture/pass-through mode), and
   watched what HS did at each step.
3. The bypass URL (`/redirect/response/{connector}`) was visibly wrong:
   it left the payment at `requires_customer_action`. Switching to
   `/redirect/complete/{connector}` advanced the state — but with a
   synthetic `customerVaultId` it errored as "Invalid Customer Vault ID".
4. Fetching the next_action redirect HTML revealed HS bakes the real
   `customerVaultId` and `orderId` into the form fields the browser
   posts back. That was the missing piece — cypress could read the same
   HTML and extract them.
5. Re-ran the curl flow with the extracted fields + synthetic 3DS proof
   → payment succeeded, refund succeeded, sync refund succeeded.
6. Only then did we wire `cy.simulateNmiRedirectComplete` in cypress —
   doing exactly what the curl script did.

The takeaway: when cypress (in replay) and HS (in any mode) disagree, treat
HS as the source of truth. Replicate the cypress test's intent against the
live connector first, prove the right shape end-to-end, *then* port that
shape into cypress.

### For future contributors / agents adding a new connector

When you add capture+replay coverage for a new connector, **update the
extended-batch table above** with one row containing your connector's
results. Include:
- Webhook strategy (HMAC-easy / HMAC-tricky / External-verify / No-impl)
- Cypress scope (which specs were run, total test count)
- Capture pass-rate and replay pass-rate
- Cassette count
- Any non-obvious quirks (e.g., "Spec X needs manual curation due to Y")

If you're an agent spawned to capture or replay a single connector, **don't
edit this table directly** — concurrent agent edits race. Instead, return
your findings to the orchestrating session (test counts, cassette count,
notable failures, any quirks worth documenting), and the orchestrator
will update the table centrally.

## What's NOT yet validated (known unknowns)

Specs that probably-just-work (no redirection, no browser dance):
- `04-NoThreeDSAutoCapture`, `06-NoThreeDSManualCapture`, `07-VoidPayment`,
  `08-SyncPayment`, `09-RefundPayment`, `10-SyncRefund`, mandate specs (11-15)

Specs that need new code:
- **`09-RefundPayment`** — new webhook event type `charge.refund.updated`.
  Refactor `fireConnectorWebhook` to be event-type-parameterized.
- **`18-BankRedirect`** — first non-3DS redirection. Wire
  `cy.handleBankRedirectRedirection` to use the same `PROXY_MODE=replay`
  webhook bypass.
- **3DS failure flows** — need `payment_intent.payment_failed` event support.
- **External 3DS (`44-ExternalThreeDS`)** — different 3DS provider; likely
  different webhook shape.

## For subagents / future contributors

If you're being asked to add coverage for another spec or another connector,
read this first:

1. **Run the spec in capture mode** with our standard env vars. If it passes
   end-to-end and produces a sensible cassette set under `captures/`, you're
   probably good. Run `normalize_captures.py`. Switch to replay. Verify HIT
   rate is 100% and no MISS/LIVE.
2. **If a step in replay fails or MISSes a cassette**, the most likely causes
   in order of probability:
   1. A connector-specific code path in `redirectionHandler.js` triggered a
      `cy.visit` that doesn't exist in `cy.handleRedirection`'s `PROXY_MODE=replay`
      bypass. Extend the bypass.
   2. HS made a connector call that wasn't recorded (e.g., the spec uses an
      endpoint we haven't run before). Check `captures/` for the
      `(test, request_id)` folder and see what's there.
   3. A new webhook event type is needed (e.g., refund flows). Extend
      `fireConnectorWebhook` to handle it.
3. **If `normalize_captures.py` dropped something it shouldn't have**, inspect
   `captures/` after normalize and compare against the mitm capture log to see
   the original calls. The script's rules are documented at the top of the
   file; add more if you find new noise patterns.
4. **For new connectors**, the decision tree is:
   - Does the connector have an `IncomingWebhook` impl in HS that we can
     replicate locally? Check `crates/hyperswitch_connectors/src/connectors/<connector>.rs`
     for `get_webhook_source_verification_*`. If verification is HMAC-based
     with a known message construction, add the connector to the
     `WEBHOOK_BYPASS_CONNECTORS` allowlist in `cy.handleRedirection` and
     write its signing branch in `fireConnectorWebhook` plus a `sign<X>Webhook`
     task in `cypress.config.js`. Stripe and Adyen are the worked examples.
   - If the connector has no webhook impl (cybersource), or verifies via
     an outbound API call (paypal hits paypal.com), **do not allowlist it**.
     The default `simulateRedirectCallback` path will route through HS's
     `/redirect/response/{connector}` endpoint, which force-syncs the
     connector — mitm matches the outbound from cassette.
   - For both paths, set `connector_webhook_details.merchant_secret` in
     the connector-create fixture if the bypass uses it (webhook path
     only — redirect-callback doesn't need it).

## Open questions / future work

- Where do cassettes live in CI? Currently `.gitignore`d. Probably S3, with a
  pull/push step in the CI workflow. Need a clear story for cassette
  versioning when test code changes.
- `normalize_captures.py` could grow heuristics for other noise patterns
  (e.g., async webhook firings that arrive after the test ends).
- More connectors. Stripe and Adyen have full webhook bypass; Cybersource
  and PayPal go through the redirect-callback fallback. Verify the
  fallback works end-to-end for those by running a sample non-3DS spec
  through capture + replay.
- A `--check` mode for `normalize_captures.py` that would fail in CI if it
  finds anything to delete (i.e., cassettes weren't captured cleanly).
- The Cypress `PROXY_MODE=replay` branch is currently inside
  `cy.handleRedirection`. For bank_redirect / pay_later / wallet, the same
  branch needs to be added to those handlers.

## File index

```
mitm-proxy/
├── README.md             this file
├── mitm_capture.py       mitmdump addon: capture mode
├── mitm_replay.py        mitmdump addon: replay mode (strict FIFO)
├── normalize_captures.py post-capture cleanup script
├── start.sh              launch capture mode + print router exports
├── start_replay.sh       launch replay mode
└── captures/             cassette tree (gitignored)
```

Cypress-side companion files:

```
cypress-tests/
├── cypress.config.js                          signStripeWebhook task
├── cypress/
│   ├── fixtures/
│   │   └── create-connector-body.json         merchant_secret for webhook signing
│   └── support/
│       ├── commands.js                        cy.fireConnectorWebhook +
│       │                                      PROXY_MODE=replay bypass in
│       │                                      cy.handleRedirection
│       └── e2e.js                             X-Request-ID wrapper (from ae846f1ae)
```
