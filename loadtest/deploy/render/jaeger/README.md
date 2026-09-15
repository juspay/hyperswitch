# Publicly hosted Jaeger for loadtest traces

A real Jaeger UI, reachable by anyone with the link, on free hosting.

```
Netlify  ──  serves traces/*.json + manifest.json        (static, permanent)
   │
   └──►  Render  ──  runs Jaeger; on every cold start it fetches the
                     manifest, pulls each trace, POSTs them to its own
                     OTLP endpoint, then serves the UI
```

## Why it is built this way

Render's free tier has no persistent disk and suspends the service after ~15
minutes idle, so anything written to disk is gone by the next visitor. Rather
than work around that, the traces are re-loaded from Netlify on every boot —
which makes memory storage the correct choice instead of a compromise.

Two things this sidesteps that cost days otherwise:

- **Grafana cannot share Tempo panels externally.** Its shared-dashboard feature
  explicitly does not support Tempo, and Explore links always require a login.
  Serving Jaeger directly is the only way to get a public trace waterfall.
- **Jaeger v2 exports its own telemetry to `localhost:4317`** — the port it is
  about to bind. The exporter starts before the receiver listens, retries in a
  hot loop, and startup never completes: a core pinned at ~99% and `/health`
  never answering. `service.telemetry.traces.processors: []` disables it.

## Deploy

**Netlify** — already live at `https://hs-loadtest-traces.netlify.app`. To
republish after changing the traces:

```bash
cd loadtest/deploy/netlify && npx netlify-cli deploy --prod --dir .
```

The trace bundles under `traces/` are **gitignored** — they are large and
regenerated often, so they are published straight from this directory and never
enter the repo. A fresh clone therefore cannot reproduce the site; regenerate
the bundles with the recipe below, assuming the trace is still inside Tempo's
retention window.

**Render** — New → Web Service → this repo:

| setting | value |
|---|---|
| Root directory | `loadtest/deploy/render/jaeger` |
| Runtime | Docker |
| Branch | `feat/jaeger-trace-hosting` |
| `TRACE_MANIFEST` | `https://hs-loadtest-traces.netlify.app/traces/manifest.json` |

Root directory is not optional here: left blank, Render builds the repo-root
Dockerfile — the full Rust router — which cannot finish in a free instance.

`TRACE_MANIFEST` is likewise easy to miss. The entrypoint treats it as optional
and starts an empty Jaeger without it, so a forgotten variable looks like a
healthy service serving no traces, with nothing in the logs pointing at the
cause.

## Adding traces

Export from Tempo, converting IDs from base64 to hex — OTLP/JSON requires hex
and Tempo's query API returns base64, which is otherwise a silent 400:

```bash
curl -s "http://127.0.0.1:3200/api/traces/<TRACE_ID>" -o /tmp/otlp.json
python3 - <<'PY'
import json, base64, re
src = json.load(open('/tmp/otlp.json'))
ID = {"traceId", "spanId", "parentSpanId"}
def hexify(v):
    try: return base64.b64decode(v).hex()
    except Exception: return v
def walk(o):
    if isinstance(o, dict):
        return {k: (hexify(v) if k in ID else walk(v)) for k, v in o.items()}
    if isinstance(o, list): return [walk(x) for x in o]
    return o
raw = json.dumps({"resourceSpans": walk(src.get("batches", []))}, separators=(',', ':'))
raw = re.sub(r'cs_[A-Za-z0-9_]{8,}', 'cs_<redacted>', raw)   # strip client secrets
open('/tmp/out.json', 'w').write(raw)
PY
```

Drop the result in `../netlify/traces/`, add its path to `manifest.json`, and
redeploy Netlify. Render needs no rebuild — it re-reads the manifest on its next
cold start.

Manifest entries may be relative (`/traces/x.json`); the entrypoint resolves them
against the manifest's own origin, so the bundle survives a hostname change.

## Before you publish

- **Scrub the traces.** The conversion above removes client secrets. Check for
  anything else your spans carry; these traces expose service topology, DB table
  names and code paths.
- **There is no authentication.** Anyone with the URL can read the traces *and*
  write into this Jaeger over OTLP. Never point a live collector at it. Put
  Cloudflare Access or Tailscale in front if that matters.
- **Cold start is ~1 minute** plus re-seeding time, and seeding cost grows with
  every trace in the manifest.
