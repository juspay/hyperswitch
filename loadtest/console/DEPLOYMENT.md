# EC2 deployment and capacity validation

Use one EC2 instance in the target services' VPC, one container and one admin.
Start with 16 vCPUs and 32 GiB RAM; choose the final size from generator CPU,
memory and network measurements. Run the generator on a different host from the
services under test so they do not compete for compute resources.

## Start

Mount encrypted gp3 EBS at `/srv/loadtest`. Size storage from measured bytes per
API attempt × attempts/sec × test duration and desired run retention; every
attempt is recorded, so artifacts are not small at 1,000 flows/sec. Configure
EBS IOPS/throughput from recorder commit latency and backlog, and keep free-space
alerts. Ensure `/srv/loadtest` is owned by container UID 1000.

From `loadtest/console`, prepare an ignored `.env` using `.env.example`. Inject
admin hash and service credentials from AWS Secrets Manager into this runtime
file or environment using the EC2 instance role. Do not build them into the
image. Set:

```text
ORIGIN=https://loadtest.example.com
CONSOLE_BIND_ADDRESS=0.0.0.0
CONSOLE_DATA_PATH=/srv/loadtest
```

Run `docker compose up -d --build`. The image uses the pinned Go/k6 extension and
Node 24 runtime. Change service endpoints in Settings to the services' private
DNS names; `127.0.0.1` inside a container refers to the container itself.
The shared `.env.example` defaults to local development. Set `PGHOST` and
`GRAFANA_URL` to addresses reachable from the container before deploying it.

Place an HTTPS ALB with an ACM certificate in front of port 4174. Allow inbound
4174 only from the ALB security group and allow browser ingress only from the
intended administrator network. Set health checks to `/health`, deregistration
delay to 75 seconds, and idle timeout to at least 90 seconds. SSE sends a snapshot
every second and does not depend on the browser remaining open.

Only one console process/container may use the volume. Jobs and sessions persist
in `/data/console.sqlite`; jobs left active after restart become interrupted.
New jobs wait until surviving workers have stopped after a restart.
Cancellation and shutdown allow 60 seconds for k6 to stop and drain. The container
grace period is 75 seconds. Forced termination leaves incomplete recording.

## Grafana

The console automatically detects root-hosted Grafana and rewrites its frontend
paths under the authenticated `/grafana/` proxy. Existing Grafana deployments
can stay at `/`; no deployment changes are required for path compatibility.
Embedding still requires Grafana to permit frames and a server identity that can
authenticate panel HTML. Grafana Live WebSocket panels are not supported.

For a dedicated Grafana deployment, you can alternatively configure:

```ini
[server]
root_url = https://loadtest.example.com/grafana/
serve_from_sub_path = true
[security]
allow_embedding = true
cookie_secure = true
```

Supply `GRAFANA_TOKEN` for a Viewer service account. Grafana UI versions that do
not accept bearer authentication for embeds require auth proxy mode:

```ini
[auth.proxy]
enabled = true
header_name = X-WEBAUTH-USER
header_property = username
auto_sign_up = true
enable_login_token = false
# Restrict to the console host's private IP; never expose this identity header publicly.
whitelist = <console-private-IP>
[users]
auto_assign_org_role = Viewer
```

Set `GRAFANA_USER` to that Viewer identity. Supply the private `GRAFANA_URL` as
the upstream address. The console strips browser cookies/identity headers and
accepts only the configured upstream. Add panel URLs under `/grafana/d-solo/`.
The proxy supports read-only dashboard/data-source queries, not Grafana editing
or live WebSocket panels.

## Acceptance

1. Run 10 flows/sec locally using the three-flow 50/30/20 mix and a Stripe test
   connector. Check login, SSE progress, cancellation, restart recovery and recon.
2. On EC2 step through 100, 500 and 1,000 offered flows/sec. Tune VU multipliers
   from full-flow latency and observed drops, rather than request latency alone.
3. Sustain 1,000 for 30 minutes in both merchant modes. Require no recording gaps,
   bounded queue, no dropped iterations due to the generator, and responsive UI.
   Report successful TPS and downstream failures separately from offered TPS.
4. Characterize recording with delayed polling at twice the configured mix's
   API-attempt rate using the mock benchmark below. Choose `--rps` after measuring
   attempts per flow; do not equate flows/sec with responses/sec.

```bash
node loadtest/k6/scenario-mix/benchmark.mjs --config loadtest/loadtest.config.example.json --rps 2000 --seconds 30 --polls 3 --output /data/benchmarks/recorder
```

Run again with `--config loadtest/k6/scenario-mix/config.example.json` to exercise
the saved-card and baseline delayed-polling paths in the six-flow mix. Inside the
container these scripts live under `/app/loadtest`.

Record enabled/disabled generator CPU time, memory/network usage, peak recorder
queue, commit latency and successful TPS. The existing benchmark's <5% CPU
overhead check is a target, not an established result. EC2 capacity is accepted
only after the sustained test passes. AWS provisioning is separate from this image.

References: [SvelteKit Node deployment](https://svelte.dev/docs/kit/adapter-node),
[k6 capacity guidance](https://grafana.com/docs/k6/latest/testing-guides/running-large-tests/),
and [Grafana auth proxy](https://grafana.com/docs/grafana/latest/setup-grafana/configure-access/configure-authentication/auth-proxy/).


## Worker ownership

The container entrypoint runs an SSR server and an independent loopback-only,
authenticated worker service. `/data/console.sqlite` stores versioned profiles,
fixture sets, jobs and activity; private worker connection metadata also lives
under `/data`. Keep this volume on one local EBS filesystem, never share it
between multiple console containers. Browser disconnects and SSR restarts leave
workers executing. Container termination signals both services and drains the
recorder; a worker crash marks unfinished runs incomplete on recovery.

Configure an existing PostgreSQL account with SELECT permission on
`payment_intent`, then use Settings → Database → Test connection. No role is
created automatically. Saved connection settings override environment defaults.
