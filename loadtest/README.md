# Payment load-test automation

Automation for non-modular and payment-method-modular payment-confirm load tests. It manages a complete local environment and can run merchant, fixture, and k6 workflows against an already provisioned cloud load-test slice.

Use the recipes in `loadtest/Justfile`; do not invoke the implementation scripts directly.

## Local prerequisites

- `just`, Node.js, k6, Git, and curl
- Podman with Compose support
- Nix for the default Hyperswitch build and migration commands
- Free ports and valid CPU sets from `deploy/config.yaml`

## Quick start

Run from `loadtest/`:

```bash
cp deploy/config.example.yaml deploy/config.yaml
cp runner/config.example.yaml runner/config.yaml

just deploy-preflight
just deploy-ready
just runner-preflight
just runner-fixtures
just runner-start
```

`runner-start` prints the Grafana dashboard URL and a p50/p75/p90/p99 latency summary.

Stop everything without deleting state:

```bash
just deploy-down
```

## Configuration

Local runs use two configuration files:

| File | Owns |
| --- | --- |
| `deploy/config.yaml` | Repositories, images, builds, endpoints, CPU affinity, state services, generated TOMLs, migrations, and observability. |
| `runner/config.yaml` | Merchant path, payment scenario, load shape, fixture policy, and local setup credentials. |

In local mode, the runner reads service endpoints from deployment configuration. Do not duplicate endpoints in runner configuration. Cloud runner-only mode defines remote targets directly and does not require deployment configuration.

Canonical application names are `router`, `modular-pm`, `vault`, `encryption`, and `superposition`. They are used for configuration keys, container labels, Loki labels, commands, and Grafana legends. Application container names are derived as `hs-<service>`.

### Application sources

Every application declares `source.path` and `source.git_url`. Missing repositories are cloned automatically.

| Mode | Behavior |
| --- | --- |
| `local` | Reuse the configured local image or build it when missing. |
| `cloud` | Pull the configured image; retain the repository for preparation and migrations. |

Set `build.force: true` or run `FORCE=1 just deploy-build` to rebuild local images.

These source modes belong to the local Podman deployer. `source.mode: cloud` does not target a remote environment; remote execution uses the runner's `environment.mode: cloud` configuration.

### Preparation and state

- PostgreSQL and Redis are shared; schemas and prefixes isolate service data.
- Router and modular-pm intentionally share the Hyperswitch schema.
- Preparation resolves override placeholders from `deploy/config.yaml` and merges the resulting TOMLs.
- Vault preparation generates one RSA key pair shared with router and modular-pm.
- `cpuset` pins each container to configured logical CPUs.
- Structured JSON logs are persisted under `deploy/logs/` and shipped to Loki.

## Deployment commands

```bash
just deploy-ready                         # complete deployment
just deploy-status                        # containers, health, and affinity
just deploy-build router modular-pm       # build selected services
just deploy-restart router modular-pm     # restart selected services
just deploy-logs router                   # application logs
just deploy-observability-logs             # Grafana/Loki/Prometheus logs
just deploy-restart-observability          # reprovision observability
just deploy-down                           # stop the stack
just deploy-reset                          # remove managed state
```

`deploy-ready` acquires repositories, resolves images, prepares configuration, starts state services, initializes state, starts applications, runs migrations, and starts observability.

## Runner model

The runner performs three stages:

1. Create or reuse a merchant and configure modular routing through Superposition APIs.
2. Create run-owned customers, PM sessions, and payments using k6 `shared-iterations`.
3. Confirm fixtures at the configured rate using k6 `constant-arrival-rate`.

Fixtures belong to one run and cannot be reused after confirmation.

```bash
just runner-fixtures     # setup and prepare fixtures
just runner-start        # execute measured traffic
just runner-status       # inspect active run
just runner-discard      # discard unused active fixtures
```

### Merchant paths

| Path | Request flow |
| --- | --- |
| `non_modular` | Payment create, then payment confirm. |
| `modular` | PM session create, payment create, PM session confirm, then payment confirm using its token. |

For modular cloud runs, combined latency is PM-session-confirm latency plus the Router `x-hs-latency` value (Router latency excluding connector time).

### Scenarios

| Scenario | Behavior |
| --- | --- |
| `guest` | No customer and no saved card; modular PM session uses volatile storage. |
| `cit_on_session` | Customer-initiated save-card flow with `setup_future_usage: on_session`. |
| `cit_off_session` | Customer-initiated save-card flow with `setup_future_usage: off_session`. |
| `cit_metadata_changed` | Non-modular CIT flow that saves a card during fixture setup and changes its metadata during measured confirm. |

Select a flow in `runner/config.yaml`:

```yaml
scenario:
  merchant_path: modular
  name: cit_off_session
```

### Load shape

Fixed 5 RPS for one minute:

```yaml
load:
  starting_rps: 5
  target_rps: 5
  step_rps: 0
  hold_seconds: 60
  idle_seconds: 0
```

For a ramp, increase `target_rps`, set `step_rps`, and optionally add `idle_seconds`. With `fixtures.count: auto`, the runner creates enough fixtures for all scheduled requests. Fixture concurrency affects preparation only.

## Smoke tests

```bash
just smoke non_modular guest
just smoke modular cit_off_session
just runner-test
just e2e-smoke
```

Smoke tests use the same merchant setup, fixture, and confirmation implementation as load tests.

## Observability

Grafana defaults to `http://127.0.0.1:3002`. The provisioned dashboard contains only:

- Request rate for router, modular-pm, vault, and encryption.
- Five-second rolling server latency percentiles and modular combined latency.
- CPU cores consumed by each application service.

Promtail discovers managed containers through canonical Podman labels. Restart observability after editing provisioned dashboards.

## Cloud runner

Cloud mode targets an environment that has already been provisioned and wired by the infrastructure repository. It does not deploy Kubernetes releases, create databases, or configure Router, PM Modular, locker, or encryption application TOMLs.

The runner host needs `just`, Node.js, k6, and network access to every configured target. Local Podman, Nix, PostgreSQL, Redis, and CPU-affinity configuration are not required.

Create the machine-local configuration:

```bash
cp runner/config.cloud.example.yaml runner/config.cloud.yaml
export LOADTEST_MERCHANT_ID=merchant_...
export LOADTEST_MERCHANT_API_KEY=...
export LOADTEST_PUBLISHABLE_KEY=pk_...
export LOADTEST_PROFILE_ID=profile_...
export LOADTEST_ORGANIZATION_ID=org_...
```

Each target has an independent base URL, API prefix, health URL, and header map. This allows a shared ingress to route Router and PM Modular with different `x-feature` values. Target headers are applied to administrative setup, fixtures, and measured k6 traffic.

The default cloud example uses an existing merchant and preconfigured Superposition context, so preflight and test execution do not mutate administrative or runtime configuration. `merchant.mode: create` is available when explicitly configured with `payments_api.admin_api_key` or `admin_api_key_env`. `superposition.mode` supports `manage`, `preconfigured`, and `disabled`.

Run preflight before any traffic. It validates the configuration and probes every required target using that target's headers:

```bash
CONFIG=runner/config.cloud.yaml just runner preflight
```

Run one smoke iteration matching the scenario in the cloud config, then verify its request IDs in the intended custom service logs:

```bash
CONFIG=runner/config.cloud.yaml just runner smoke modular cit_off_session
```

After proving Router, database, PM Modular, locker, and encryption isolation:

```bash
CONFIG=runner/config.cloud.yaml just runner fixtures
CONFIG=runner/config.cloud.yaml just runner start
CONFIG=runner/config.cloud.yaml just runner status
```

`fixtures.wait_before_start_seconds` sets a minimum settling period after fixture preparation. `runner start` waits only for the remaining time, so a manual delay between `fixtures` and `start` is counted toward it.

Cloud Router targets request `x-hs-latency: true`. Runner results report `hyperswitch_internal_excluding_connector`, the Router response-header latency excluding connector time; `payment_confirm` remains the client-observed k6 duration. `combined` adds the internal Router latency to PM-session-confirm latency.

`runner status` reports aggregate latency and a latency breakdown for each configured RPS phase, including the first and last measured request timestamps for that phase.

Non-modular customer scenarios still use the PM customer API during fixture preparation, so they require a PM target even though measured payment-confirm traffic is non-modular. A non-modular guest scenario requires only Router.

The runner uses the PM API prefix configured under `targets.modular_pm.api_prefix`. The current sandbox load-test ingress exposes the runner's modular flows under `/v1`; keep cloud configurations on `/v1` unless the deployed ingress explicitly exposes another version.

Cloud infrastructure remains responsible for provisioning the custom instances, routing `x-feature` traffic, assigning the custom database, and wiring downstream endpoints through application TOML or Helm configuration.

## Adding a flow

1. Define its customer, storage, and save-card behavior in `runner/lib/scenarios.js`.
2. Add prerequisite requests to `runner/k6/fixtures.js` and measured requests to `runner/k6/workload.js` only when existing behavior cannot be reused.
3. Add a focused test and include the flow in `e2e-smoke`.

Read service endpoints from deployment configuration; never duplicate them in a flow.

## Troubleshooting

| Symptom | Action |
| --- | --- |
| Connection refused | Run `just deploy-status`, then `just deploy-logs <service>`. |
| Partial fixture creation | Fix the first reported API failure, run `just runner-discard`, then create fresh fixtures. |
| Invalid or expired token | Fixtures were consumed or expired; run `just runner-fixtures` again. |
| Modular path bypassed | Check Superposition health and ensure propagation wait covers router polling. |
| Grafana has no request data | Check JSON logging, Promtail/Loki health, and dashboard time range. |
| Grafana has no CPU data | Check Podman exporter and Prometheus; restart observability after dashboard changes. |
| Migration failed | Verify repository path, database URL, schema, and printed migration command. |

Machine-local configs, generated TOMLs/keys, logs, and runner state must not be committed. Commit example configs and minimal override fragments only.
