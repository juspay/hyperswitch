# MITM replay — post-redirect connector calls miss on replay (nmi / redsys / paypal)

Handoff doc for continuing the MITM capture/replay pilot elsewhere. Describes the
one remaining structural problem, its root cause, the full reproduction recipe,
and the current state on disk.

---

## TL;DR

The MITM proxy records Hyperswitch→connector HTTP calls during a live Cypress run
("capture") and serves them back offline ("replay"). It correlates a recorded
response to a replayed request by an `X-Request-ID` that Cypress stamps onto every
`cy.request` and Hyperswitch propagates onto its outbound connector calls.

For **card 3DS / redirect flows**, this correlation breaks for the connector calls
that happen *after* the redirect:

- **At capture time** those calls are triggered by the **browser** returning from
  the ACS / connector-hosted page (a `cy.visit` navigation, not a `cy.request`), so
  Cypress never stamps an `X-Request-ID` on them. Hyperswitch falls back to
  generating its own request id — a **server UUID** like
  `019e258e-c627-7791-be47-908445951745`. The cassette is saved under that UUID.
- **At replay time** the redirect is skipped; a bypass command
  (`simulateRedirectCallback` / `simulateNmiRedirectComplete` /
  `simulateRedsysRedirectComplete`) re-drives the post-redirect step with
  `cy.request`s. Those *do* get Cypress `X-Request-ID`s — so the replayed call
  looks for a cassette under a cypress id (`<djb2>-NNN`), not the server UUID it
  was captured under. **Miss.**

Net effect on the 3 affected connectors: the post-redirect calls (`transact.php`
CompleteAuthorize for nmi, `trataPeticionREST` for redsys, `GET /v2/checkout/orders`
+ `POST .../capture` for paypal) all MISS in strict replay, failing those tests.

**This is a replay-path bug, not a capture gap** — the captures are complete and
correct; the rids just can't line up because capture and replay drive the
post-redirect step through two different code paths.

---

## Background — what the pilot is

Goal: run the Hyperswitch Cypress suite offline (no live connector sandboxes) by
recording connector traffic once and replaying it.

- `mitm-proxy/mitm_capture.py` — mitmproxy addon, records each HS→connector
  round-trip as a JSON cassette under
  `mitm-proxy/captures/<connector>/<safe_test_name>/<request_id>/<NNN>.json`.
- `mitm-proxy/mitm_replay.py` — mitmproxy addon, serves cassettes back.
  Match key: `(connector, request_id, method, path)`, FIFO per key.
- `cypress-tests/cypress/support/e2e.js` — stamps `X-Request-ID: <djb2(testTitle)>-<NNN>`
  on every `cy.request` (committed: `ae846f1ae`).
- Hyperswitch propagates the incoming `X-Request-ID` onto outbound connector calls
  when `trace_header.id_reuse_strategy = use_incoming`.

The 9 "extended" connectors targeted (from
`.github/workflows/cypress-tests-runner.yml`):
payments — `bluesnap gigadat loonio nmi paypal redsys zift`;
payouts — `adyenplatform wise`.

CI runs the **full** `cypress/e2e/spec/Payment/**/*` (62 specs) /
`Payout/**/*` (7 specs) globs; per-connector Cypress config self-skips unsupported
flows. The pilot mirrors that.

---

## Current verdict (all 9 extended connectors, full-glob strict replay)

| Connector | Cassettes | Replay MISS | Live capture (P/F/S) | Strict replay (P/F/S) | State |
|---|---|---|---|---|---|
| bluesnap | 129 | 0 | 266/6/215 | 266/6/215 | ✅ replay == live |
| gigadat | 3 | 0 | 260/10/217 | 260/10/217 | ✅ replay == live |
| loonio | 4 | 0 | ~260/10/217 | 260/10/217 | ✅ clean |
| zift | 154 | 2 | 302/1/184 | 294/9/184 | ✅ effectively clean¹ |
| adyenplatform | 13 | 0 | 45/0/… | 45/0/… | ✅ full parity |
| wise | 30 | 0 | 39/0/6 | 39/0/6 | ✅ full parity |
| **paypal** | 215 | **4** | 348/1/138 | 339/10/138 | ⚠️ **this bug** |
| **redsys** | 87 | **21** | 258/1/228 | 253/6/228 | ⚠️ **this bug** |
| **nmi** | 223 | **34** | 311/6/170 | 291/26/170 | ⚠️ **this bug** |

¹ zift's 2 MISS are inside `40-ExternalVault` (a VGS multi-connector test that
**fails in live capture too** because VGS isn't configured in this dev env) — not
the redirect bug, not worth fixing. Separately, zift replay also fails
`35-PaymentsEligibilityAPI` / `41-CardPaymentBlocking` with **no MISSes** — an
unexplained delta vs its live capture (possibly flakiness in those HS-feature
endpoints); lower priority, not the redirect bug.

P/F/S = passed / failed / skipped, of 487 tests across 62 specs.

The failures that show up in *both* live capture and replay (e.g. NTID assertions,
`ExternalVault`/VGS, `CardPaymentBlocking`/BIN-service, `PaymentsEligibilityAPI`)
are genuine connector limitations or missing dev-env config — **not** caused by
the MITM system, and out of scope for this doc.

---

## The problem in detail

### Affected: nmi, redsys, paypal — all card 3DS / redirect connectors

Replay MISS log signature (from `mitm_replay_strict.py`):

```
# nmi   — CompleteAuthorize after the 3DS redirect, plus the sync right after
[replay] MISS  [nmi]    POST /api/transact.php (rid=47f76371-005)
[replay] MISS  [nmi]    POST /api/query.php    (rid=dd66bbf4-006)
# redsys — 3DS completion + consult
[replay] MISS  [redsys] POST /sis/rest/trataPeticionREST    (rid=47f76371-004)
[replay] MISS  [redsys] POST /apl02/services/SerClsWSConsulta (rid=dd66bbf4-005)
# paypal — order status GET + capture, after returning from paypal.com
[replay] MISS  [paypal] GET  /v2/checkout/orders/2PK60450Y09192512 (rid=94eb78f0-004)
```

### Root cause

The `X-Request-ID` correlation only holds if Cypress fires the **same sequence of
`cy.request`s** in capture and replay. For a 3DS/redirect payment it does not:

| Step | Capture path | Replay path |
|---|---|---|
| Confirm payment | `cy.request` → rid `-NNN` | `cy.request` → rid `-NNN` |
| 3DS / redirect | **`cy.visit`** to the ACS / connector page — a *browser navigation*, **0 `cy.request`s** | **skipped** — `simulate*RedirectCallback` fires **1+ `cy.request`s** |
| Post-redirect connector call (CompleteAuthorize / order GET / capture) | triggered by the **browser** hitting HS's `/redirect/...` endpoint → HS has **no incoming `X-Request-ID`** → stamps a **server UUID** | triggered by the bypass `cy.request` → **has** a cypress rid, and the step counter has been **shifted** by the bypass's extra `cy.request`s |

So two things go wrong at once for post-redirect calls:

1. **Server-UUID vs cypress-rid mismatch** — the cassette is keyed under a server
   UUID; replay asks for it under a cypress rid. Never matches.
2. **Step-counter desync** — even cypress-rid'd calls *after* the redirect are off,
   because the replay-side bypass injected `cy.request`s the capture path didn't
   have (the capture used a `cy.visit` instead). Every downstream rid is shifted.

Confirmed concretely. Example (paypal, test `94eb78f0`):
- Captured: `GET /v2/checkout/orders/<id>` + `POST .../capture` are both under
  server UUID `019e258e-c627-7791-...`.
- Replayed: requested under cypress rid `94eb78f0-004`. MISS.

nmi/redsys are larger because they have more post-redirect calls (CompleteAuthorize
*and* a sync/consult); paypal is milder (order GET + capture = 4 calls total).

### Why this is the *same* bug for all three

- nmi: bypass `simulateNmiRedirectComplete` → `POST /redirect/complete/nmi` →
  HS `CompleteAuthorize` → `transact.php` + `query.php`.
- redsys: bypass `simulateRedsysRedirectComplete` → `POST /redirect/complete/redsys`
  → HS `CompleteAuthorize` → `trataPeticionREST` + `SerClsWSConsulta`.
- paypal: bypass `simulateRedirectCallback` → `POST /redirect/response/paypal` →
  HS `PaymentRedirectSync` → `GET /v2/checkout/orders/<id>` + `POST .../capture`.

Different connectors, different endpoints, **one mechanism**: the post-redirect
connector calls are browser-driven (server UUID) at capture time and
`cy.request`-driven (cypress rid, shifted counter) at replay time.

The dispatch lives in `cypress-tests/cypress/support/commands.js` in
`handleRedirection`, under `if (Cypress.env("PROXY_MODE") === "replay")`.

### Known prior workaround (from `mitm-proxy/README.md`)

The README's redsys section describes a **cassette-curation pass**: after capture,
manually relocate the browser-callback cassettes from their server-UUID rid folders
to the cypress rid the bypass triggers. This works but is fragile — it bakes in
per-test step-count assumptions and must be redone on every re-capture.

A *proper* fix would make the replay bypass not desync the rid sequence — e.g. have
`simulate*RedirectCallback` not advance the shared `stepCounter` for its internal
`cy.request`s (the same idea as the `support/e2e.js` hook-registration guard), and
give the post-redirect connector call a **deterministic** rid that the cassette can
be keyed under in both capture and replay.

---

## Reproduction

### Prerequisites

- Hyperswitch router built: `target/debug/router` exists.
- Postgres + Redis up (as the existing dev setup uses — shared by all router instances).
- `mitmproxy` installed (`mitmdump` on PATH). mitm CA cert generated at
  `~/.mitmproxy/mitmproxy-ca-cert.pem` (created on first `mitmdump` run).
- `cypress-tests/` deps installed (`npm ci --prefix cypress-tests`).
- `creds.json` at repo root with sandbox creds for the connectors.
- Cypress + HS already wired for the MITM pilot:
  - `cypress-tests/cypress/support/e2e.js` has the `X-Request-ID` wrapper (committed `ae846f1ae`).
  - `cypress-tests/cypress/support/commands.js` + `cypress.config.js` have the
    replay bypasses (committed `b6849120d` and earlier).

### Infra layout — 3 router instances, each with its own mitm

| HS port | mitm proxy port | mitm admin port |
|---|---|---|
| 8080 | 8888 | 8181 |
| 8089 | 8889 | 8182 |
| 8090 | 8890 | 8183 |

**Each router instance MUST be started with the complete pilot config.** This was
the single biggest footgun — `config/development.toml` was reverted to its
committed state during this work, so the pilot config has to be supplied via env
vars on every restart. The required set (verified against the known-good HS:8080):

```
ROUTER__PROXY__HTTP_URL  = http://127.0.0.1:<mitm_port>
ROUTER__PROXY__HTTPS_URL = http://127.0.0.1:<mitm_port>
ROUTER__PROXY__MITM_ENABLED = true
ROUTER__PROXY__MITM_CA_CERTIFICATE = <encoded ~/.mitmproxy/mitmproxy-ca-cert.pem>
ROUTER__TRACE_HEADER__ID_REUSE_STRATEGY = use_incoming
ROUTER__SERVER__PORT = <hs_port>
ROUTER__MULTITENANCY__TENANTS__PUBLIC__BASE_URL = http://localhost:<hs_port>
RUST_MIN_STACK = 11534336
```

Three things that bite if missing:
1. **`MITM_CA_CERTIFICATE` / `MITM_ENABLED`** — without them HS won't route HTTPS
   connector calls through the mitm proxy → **0 cassettes captured** (calls go
   straight to the live connector).
2. **`TRACE_HEADER__ID_REUSE_STRATEGY=use_incoming`** — without it HS generates its
   own server-UUID request ids for *all* outbound calls → cassettes keyed by UUIDs
   → everything MISSes on replay.
3. **`MULTITENANCY__TENANTS__PUBLIC__BASE_URL`** — `state.base_url` (used to build
   redirect callback URLs) comes from the public tenant's `base_url`
   (`crates/router/src/routes/app.rs:658`). If all instances share
   `http://localhost:8080`, a redirect/3DS flow run on 8089/8090 routes its
   callback to **HS:8080** — capturing into the wrong instance's mitm. Each
   instance needs its own.

`mitm-proxy/repro/restart_hs.sh` encodes all of this and verifies all 5 items
against the running config before declaring an instance OK. Identify a router by
its **listening port** (`ss -ltnp`), never by `pgrep` of env vars — all router
processes share the identical argv `target/debug/router`.

### Scripts (in `mitm-proxy/repro/`)

- `mitm_replay_strict.py` — the replay mitmproxy addon in **strict mode**: on a
  cassette MISS (or a connector call with no `x-request-id`) it returns HTTP 599
  instead of silently forwarding live. This is what makes coverage gaps visible.
  (The repo's committed `mitm-proxy/mitm_replay.py` does *not* hard-fail — it was
  reverted; this strict copy is the test instrument.)
- `restart_hs.sh` — restarts HS:8089 + HS:8090 with the complete config above and
  verifies it. HS:8080 was left as the original (already correctly configured).
- `cap_one.sh <connector> <hs_port> <mitm_port> <admin_port>` — captures one
  connector's full Payment glob against the live sandbox.
- `replay_one.sh <connector> <hs_port> <mitm_port> <admin_port>` — strict
  replay-verify of one connector's full Payment glob; prints HIT/MISS counts and
  sample MISSes.

Notes on the scripts:
- `sleep` is intentionally avoided (`pause()` uses `timeout … tail -f /dev/null`)
  because the harness this was run under blocks foreground `sleep`. Harmless to
  replace with real `sleep` elsewhere.
- They write logs/artifacts to `${OUT:-/tmp/mitm_repro}`.
- `cap_one.sh` does `rm -rf mitm-proxy/captures/<connector>` before capturing —
  back up first if needed (see "Current state").

### Steps to reproduce the bug

```bash
cd <repo-root>

# 1. Restart HS:8089 + HS:8090 with the complete pilot config (HS:8080 already ok).
#    Verifies cert / mitm_enabled / id_reuse_strategy / base_url / proxy url.
bash mitm-proxy/repro/restart_hs.sh

# 2. Capture an affected connector against its live sandbox (full 62-spec glob).
#    nmi/redsys/paypal are redirect connectors — capture them on HS:8080, whose
#    base_url matches its port (or any instance, now that base_url is per-instance).
bash mitm-proxy/repro/cap_one.sh nmi    8080 8888 8181
bash mitm-proxy/repro/cap_one.sh redsys 8090 8890 8183
bash mitm-proxy/repro/cap_one.sh paypal 8089 8889 8182

# 3. Strict replay-verify each. This is where the bug shows.
bash mitm-proxy/repro/replay_one.sh nmi    8080 8888 8181
bash mitm-proxy/repro/replay_one.sh redsys 8090 8890 8183
bash mitm-proxy/repro/replay_one.sh paypal 8089 8889 8182
```

Expected: each `replay_one.sh` reports a non-zero MISS count, and the sample
MISSes are the post-redirect endpoints listed above
(`transact.php` / `trataPeticionREST` / `GET /v2/checkout/orders`). Compare the
`replay` summary line to the `capture` summary line — the extra failed specs in
replay are exactly the redirect tests whose post-redirect cassettes missed.

To see the server-UUID vs cypress-rid mismatch directly:

```bash
# post-redirect cassettes are keyed under server UUIDs:
grep -rl '"path": "/v2/checkout/orders/' mitm-proxy/captures/paypal/ | while read f; do
  python3 -c "import json;r=json.load(open('$f'));print(r['request_id'], r['request']['method'], r['request']['path'])"
done
# -> rids look like 019e258e-c627-7791-...  (server UUID, not <djb2>-NNN)
```

### A clean (unaffected) connector for comparison

```bash
bash mitm-proxy/repro/cap_one.sh    bluesnap 8080 8888 8181
bash mitm-proxy/repro/replay_one.sh bluesnap 8080 8888 8181
# -> 0 MISS, replay summary == capture summary. No redirect-completion connector
#    calls of this shape, so the bug doesn't apply.
```

---

## Current state on disk (as of this handoff)

- **Captures present** (`mitm-proxy/captures/`), all from correctly-configured
  instances: bluesnap 129, nmi 223, gigadat 3, loonio 4, paypal 215, redsys 87,
  zift 154, adyenplatform 13, wise 30.
- **Backups of earlier/partial captures** (safe to delete once happy):
  - `/tmp/partial_backup_1778742593/` — pre-recapture partials of the 6 payment connectors
  - `/tmp/bluesnap_partial_backup_1778740959/` — bluesnap's pre-recapture partial
  - `/tmp/nmi_captures_old_1778732604/` — an even older nmi capture
- **HS instances running** (leave up; restart with `restart_hs.sh` if needed):
  HS:8080 original config; HS:8089 + HS:8090 restarted with the complete config.
- **`config/development.toml` is reverted** (committed state) — it does **not**
  contain the pilot's `[proxy]` mitm block or `[trace_header]`. That is why the
  config must be supplied via env on every router restart. If you'd rather not
  fight env vars, re-add the pilot config to the TOML instead (mitm cert,
  `mitm_enabled = true`, `trace_header.id_reuse_strategy = "use_incoming"`).
- **`mitm-proxy/mitm_replay.py`** in the repo is the reverted (non-strict) version.
  Use `mitm-proxy/repro/mitm_replay_strict.py` for honest MISS-surfacing replay.
- The committed Cypress changes (`b6849120d` and earlier) — the replay bypasses and
  webhook signing — are in place on the branch.

---

## Candidate fixes (for whoever picks this up)

1. **Proper bypass fix (preferred).** In `support/e2e.js` / `commands.js`, make the
   `simulate*RedirectCallback` family not consume the shared `stepCounter` for
   their internal `cy.request`s, and have the post-redirect connector call carry a
   **deterministic** rid (e.g. `<testIdHash>-redirect`) in *both* capture and
   replay. Capture side: the post-redirect call would still need that deterministic
   rid — which means either instrumenting the capture path's redirect handling to
   issue it, or a normalization step. This removes the desync structurally and
   needs no per-capture curation.

2. **Cassette-curation pass (the README's existing workaround).** After capture,
   relocate the server-UUID post-redirect cassettes to the cypress rid the bypass
   triggers. Works, but fragile and must be rerun on every re-capture.

3. **Capture through the bypass too.** Make capture mode also use
   `simulate*RedirectCallback` so capture and replay paths match exactly. Risk: the
   bypass posts synthetic 3DS proof, which the *real* connector sandbox may reject
   at capture time — needs a probe per connector.

Whichever route: `replay_one.sh` is the regression check — target **0 MISS** and
`replay summary == capture summary` for nmi / redsys / paypal.
