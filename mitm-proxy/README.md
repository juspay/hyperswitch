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

### Current pilot work (Stripe 3DS)

Three new things, captured in the current uncommitted diff:

#### A. Replay-mode webhook bypass — `cypress-tests/cypress/support/commands.js`

When `CYPRESS_PROXY_MODE=replay`, `cy.handleRedirection` **skips the browser
3DS dance entirely** and instead fires a signed connector webhook directly to
HS. HS verifies the signature, runs its
`CallConnectorAction::HandleResponse(payload)` path (no connector outbound),
and uses the webhook body as the response. The payment advances to terminal
state without any browser navigation or 3DS challenge.

Implementation:
- `cy.fireConnectorWebhook(globalState)` — builds a Stripe-formatted
  `payment_intent.succeeded` (or `.amount_capturable_updated` for manual-capture
  flows) webhook, signs it with HMAC-SHA256, POSTs to
  `${baseUrl}/webhooks/${merchantId}/${connectorId}`.
- `cy.task("signStripeWebhook")` (in `cypress.config.js`) does the HMAC
  Node-side because the browser context lacks `node:crypto`.
- The connector account's webhook secret comes from
  `cypress/fixtures/create-connector-body.json` →
  `connector_webhook_details.merchant_secret`. HS stores this on the
  merchant-connector-account and uses it for signature verification.

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
Drops two kinds of cassette that pollute the dir but mustn't be replayed:

1. **Server-UUID folders.** During capture, the 3DS browser dance causes the
   ACS form to POST back to HS without carrying our `X-Request-ID`. HS mints a
   fresh UUID for that incoming and propagates it to its connector sync
   outbound. Cypress's bypass (A) skips the whole browser dance in replay, so
   nothing ever asks for these cassettes.
2. **Per-(method, path) duplicates within a single Cypress `request_id`.** See
   [The cy.visit duplicate problem](#the-cyvisit-duplicate-problem) below.

The script keeps only the cassette with the latest `captured_at` in each
duplicate group. Replay-side matcher stays strict FIFO.

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

### Why we chose normalize-captures over alternatives

Options considered:

| Approach | Verdict |
|---|---|
| **LIFO at replay** | Empirically works because HS uses "latest wins" state, but it's a heuristic that could fail silently if any HS field used "first wins". |
| **Pause mitm recording during cy.visit** | Doesn't help: mitm is a transparent proxy, pausing only skips writing the cassette to disk. HS still makes the real call and updates its state. |
| **`cy.intercept` to inject the X-Request-ID into the browser-side ACS callback** | Works in principle but cross-origin intercepts on form POSTs are finicky in Cypress 14. |
| **Skip duplicate `cy.request`s via a Node-side seen-set** | Cleanest in principle but requires caching response objects and replaying them; can break tests with non-idempotent `cy.then` handlers. |
| **`normalize_captures.py`** ✓ | Capture happens normally (every call recorded). Post-capture script deletes earlier duplicates per (method, path), keeping the latest — which is what HS's state references anyway. Cassettes on disk become the explicit source of truth. Replay matcher stays strict FIFO. |

The script is small (~140 lines), idempotent, and the cassettes it keeps
exactly match what HS expects on replay. We can hand-inspect or hand-edit them.

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

- ✅ `Payment/05-ThreeDSAutoCapture.cy.js` — single it() block, 3DS auto-capture
- ✅ `Payment/16-ThreeDSManualCapture.cy.js` — 4 contexts, 3DS manual capture with
  separate capture call

Both pass with 100% HIT rate, 0 MISS, 0 LIVE fallbacks.

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
4. **For new connectors**, you'll need to add a per-connector branch in
   `fireConnectorWebhook` (currently throws for non-Stripe), implement the
   connector's webhook payload format and signature scheme, and set
   `connector_webhook_details.merchant_secret` in the connector-create fixture
   for that connector.

## Open questions / future work

- Where do cassettes live in CI? Currently `.gitignore`d. Probably S3, with a
  pull/push step in the CI workflow. Need a clear story for cassette
  versioning when test code changes.
- `normalize_captures.py` could grow heuristics for other noise patterns
  (e.g., async webhook firings that arrive after the test ends).
- Per-connector webhook helpers — currently only Stripe is implemented.
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
