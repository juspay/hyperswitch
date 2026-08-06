# Payment load-test automation

Local automation for non-modular and payment-method-modular payment-confirm load tests. It manages repositories, images, generated TOMLs, PostgreSQL, Redis, migrations, application containers, observability, merchants, fixtures, and k6 traffic.

Use the recipes in `loadtest/Justfile`; do not invoke the implementation scripts directly.

## Prerequisites

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

There are two configuration files:

| File | Owns |
| --- | --- |
| `deploy/config.yaml` | Repositories, images, builds, endpoints, CPU affinity, state services, generated TOMLs, migrations, and observability. |
| `runner/config.yaml` | Merchant path, payment scenario, load shape, fixture policy, and local setup credentials. |

The runner reads service endpoints from deployment configuration. Do not duplicate endpoints in runner configuration.

Canonical application names are `router`, `modular-pm`, `vault`, `encryption`, and `superposition`. They are used for configuration keys, container labels, Loki labels, commands, and Grafana legends. Application container names are derived as `hs-<service>`.

### Application sources

Every application declares `source.path` and `source.git_url`. Missing repositories are cloned automatically.

| Mode | Behavior |
| --- | --- |
| `local` | Reuse the configured local image or build it when missing. |
| `cloud` | Pull the configured image; retain the repository for preparation and migrations. |

Set `build.force: true` or run `FORCE=1 just deploy-build` to rebuild local images.

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

For modular runs, combined latency is PM-session-confirm latency plus payment-confirm latency.

### Scenarios

| Scenario | Behavior |
| --- | --- |
| `guest` | No customer and no saved card; modular PM session uses volatile storage. |
| `cit_on_session` | Customer-initiated save-card flow with `setup_future_usage: on_session`. |
| `cit_off_session` | Customer-initiated save-card flow with `setup_future_usage: off_session`. |

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
