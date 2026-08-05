# Payments load-test automation

This directory owns the local payments load-test lifecycle: repositories, images, generated application configuration, state services, migrations, application containers, observability, fixtures, and direct API load generation.

## Configuration

Copy the examples once:

```bash
cp loadtest/deploy/config.example.yaml loadtest/deploy/config.yaml
cp loadtest/runner/config.example.yaml loadtest/runner/config.yaml
```

`deploy/config.yaml` is the single source of truth for repository locations, images, service endpoints, ports, CPU pinning, migrations, and inter-service configuration. `runner/config.yaml` contains only scenario and traffic settings and references the deployment config.

Repositories support local `path` sources and Git sources. Preparation creates per-application TOML overrides and required key material before containers start. Payments and PM modular share the payments schema; their configured migration recipes remain distinct.

## Local workflow

Run all commands from `loadtest/`:

```bash
just deploy-preflight
just deploy-ready
just runner-preflight
just runner-fixtures
just runner-start
```

Useful operations:

```bash
just runner-status
just runner-discard
just deploy-status
just deploy-logs payments
just deploy-down
```

`deploy-ready` performs repository acquisition, builds missing images, prepares overrides and keys, starts PostgreSQL and Redis, runs migrations, starts applications, and starts observability.

## Scenarios

The runner has two merchant paths:

- `non_modular`: payment create followed by payment confirm.
- `modular`: PM session create, payment create, PM session confirm, then payment confirm using the returned token.

Both paths support:

- `one_time`: guest payment with no saved card.
- `cit_on_session`: customer-initiated save-card flow with `on_session` future usage.
- `cit_off_session`: customer-initiated save-card flow with `off_session` future usage.

Scenarios are code-owned in `runner/lib/scenarios.js`; YAML selects a path and scenario but does not redefine API behavior. Fixtures are bound to their run and merchant and cannot be reused across scenarios.

To add a flow, add one scenario definition and its API behavior, then add focused tests. Deployment, scheduling, fixture ownership, and observability should not need flow-specific branches.

## Validation

Run the unit suite and all six API smoke paths with public recipes:

```bash
just runner-test
just e2e-smoke
```

Run one smoke path:

```bash
just smoke modular cit_off_session
```

The smoke command uses the same merchant setup, fixture creation, and confirmation engine as a load run.

## Observability

Grafana is available at `http://127.0.0.1:3002`. The provisioned **Payments Load Test** dashboard intentionally contains only request rate, 5-second rolling server latency percentiles, and service CPU usage. Loki discovers managed application containers through stable Podman labels rather than container-name patterns.

## Legacy tests

Existing k6 tests elsewhere under `loadtest/` remain available for their original use cases. They are not part of this automation runner and are not wrapped by its Just recipes.
