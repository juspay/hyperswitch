# MITM proxy — offline Cypress for Hyperswitch

This directory holds the proxy half of an experiment to **run the Hyperswitch
Cypress suite without depending on live connector sandboxes**. The Cypress
side lives in [`../cypress-tests/`](../cypress-tests/).

## Why it exists

The Cypress suite calls real connector sandboxes (Stripe, Adyen, …) on every
run. That brings flake when sandboxes hiccup, rate-limit pain, cred-rotation
toil, and slow feedback. Offline replay makes the tests hermetic, fast, and
free of live creds in CI.

## TL;DR

```bash
# capture once — runs cypress against live sandboxes, records HS→connector calls
./start.sh

# replay forever — serves cassettes back to HS instead of calling the connector
./start_replay.sh
```

Same Cypress code runs in both modes. The only difference at the test layer is
`CYPRESS_MOCK_SERVER=true` for replay, which skips browser-side 3DS while
keeping the connector-call sequence identical.

## Architecture

```
        ┌──────────┐     ┌──────────────┐     ┌──────────┐     ┌────────────┐
        │ Cypress  │ ──▶ │ Hyperswitch  │ ──▶ │ mitmproxy │ ──▶ │  connector │
        │          │     │    router    │     │          │     │  sandbox   │
        └──────────┘     └──────────────┘     └──────────┘     └────────────┘
                                                    │
                                  capture: forward + write JSON to captures/
                                  replay : serve recorded JSON, never forward
```

The router is configured with `proxy.http_url` / `proxy.https_url` pointing at
mitm and trusts mitm's CA via `proxy.mitm_ca_certificate`.

## How a call is matched to a cassette

Match key: **`(connector, x-request-id, method, path)`**.

`x-request-id` is the linchpin. Cypress's `cypress/support/e2e.js` wraps every
`cy.request` with `X-Request-ID: <djb2(testTitle)>-<NNN>` (NNN = per-step
counter). The router, configured with
`trace_header.id_reuse_strategy = "use_incoming"`, propagates that ID onto
its connector outbound. Same test → same request_id → deterministic cassette
lookup on replay.

Duplicates (same key in multiple files — e.g. Cypress retried a step) are
collapsed at load time: **last file wins**. The last file is always from the
successful run whose downstream cassettes (PSync, Capture, …) reference its
PI / resource ID.

## Files

| File | Purpose |
|---|---|
| `mitm_capture.py`  | mitmdump addon — records every HS→connector flow as a JSON cassette |
| `mitm_replay.py`   | mitmdump addon — answers each connector call with the matching cassette |
| `start.sh`         | local-dev launcher for capture mode; prints the router env exports |
| `start_replay.sh`  | local-dev launcher for replay mode; also used by CI |
| `requirements.txt` | mitmproxy + its deps |
| `captures/`        | cassette tree (gitignored) |

## Local dev

### One-time setup

1. Install `uv`: <https://docs.astral.sh/uv/>
2. Generate the mitm CA cert: run `./start.sh`, Ctrl-C immediately. Creates
   `~/.mitmproxy/mitmproxy-ca-cert.pem`.
3. Apply the printed router env exports to your HS terminal and restart the
   router.

### Capture

```bash
# Terminal 1
cd mitm-proxy && ./start.sh

# Terminal 2
cd cypress-tests
CYPRESS_CONNECTOR=stripe \
CYPRESS_BASEURL=http://localhost:8080 \
CYPRESS_ADMINAPIKEY=test_admin \
CYPRESS_CONNECTOR_AUTH_FILE_PATH=$(pwd)/../creds.json \
CYPRESS_PROXY_ADMIN_URL=http://127.0.0.1:8001 \
npx cypress run --headless --spec '<your spec>'
```

### Replay

```bash
# Terminal 1
cd mitm-proxy && ./start_replay.sh

# Terminal 2 — same cypress invocation + MOCK_SERVER=true
cd cypress-tests
CYPRESS_CONNECTOR=stripe \
CYPRESS_BASEURL=http://localhost:8080 \
CYPRESS_ADMINAPIKEY=test_admin \
CYPRESS_CONNECTOR_AUTH_FILE_PATH=$(pwd)/../creds.json \
CYPRESS_PROXY_ADMIN_URL=http://127.0.0.1:8001 \
CYPRESS_MOCK_SERVER=true \
npx cypress run --headless --spec '<your spec>'
```

Replay logs `HIT` for every cassette served; a `MISS` means the cassette set
is out of sync with the test (re-capture the affected spec).

## CI integration

CI runs replay via the same `start_replay.sh` script. The
`mitm-cassette-replay-cypress-tests` job in
[`.github/workflows/cypress-tests-runner.yml`](../.github/workflows/cypress-tests-runner.yml):

1. Downloads `cassettes/{connector}/{Platform|Payment|Payout}.tar.gz` from S3
   and extracts into `mitm-proxy/captures/`.
2. Starts `mitm-proxy/start_replay.sh` in the background.
3. Boots the router with mitm-proxy env vars (HTTPS_URL, HTTP_URL,
   MITM_CA_CERTIFICATE, ID_REUSE_STRATEGY).
4. Runs Cypress with `CYPRESS_MOCK_SERVER=true`.

To upload new cassettes for a connector, pack from inside `captures/`:

```bash
cd mitm-proxy/captures
tar -czf /tmp/Payment.tar.gz <connector>/   # or Platform/ or Payout/
aws s3 cp /tmp/Payment.tar.gz "${S3_BUCKET_URI}/cassettes/<connector>/Payment.tar.gz"
```

## Environment variables

| Var | Used by | Purpose |
|---|---|---|
| `CAPTURE_DIR` | both | Where cassettes are read/written. Defaults to `<script_dir>/captures`. |
| `ADMIN_PORT`  | both | Test-lifecycle admin server port (default `8001`). |
| `PROXY_PORT`  | start scripts | mitmdump listen port (default `8888`). |
| `CAPTURE_BASE_URLS` | capture | Comma-separated URL prefixes to capture. Empty = capture everything. |
| `CONNECTOR`   | capture | Tag override for the connector. If empty, inferred from request host. |
