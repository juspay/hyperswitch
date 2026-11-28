---
name: hyperswitch-local-setup
description: >-
  Run Hyperswitch locally: the one-click scripts/setup.sh with its
  deployment profiles, the Docker Compose development build, native Rust
  setup, health checks, logs and common failures. Use when someone wants a
  local Hyperswitch server, Control Center or Web SDK to try or develop
  against.
---

# Running Hyperswitch locally

References: `docs/try_local_system.md`, `docs/one_click_setup.md`,
`config/docker_compose.toml`.

## One-click (prebuilt images)

```bash
git clone --depth 1 --branch latest https://github.com/juspay/hyperswitch
cd hyperswitch
scripts/setup.sh
```

The script checks Docker/Podman and ports, then asks for a profile:

| Profile | Services |
|---------|----------|
| Standard (recommended) | App server, Control Center, Web SDK, PostgreSQL, Redis |
| Full | Standard + Grafana/Prometheus monitoring + scheduler |
| Standalone App Server | Server, PostgreSQL, Redis only (API testing) |

## Build from source with Compose

```bash
docker compose --file docker-compose-development.yml up -d   # compiles the router; can take ~30 min
```

For a native toolchain, follow the Rust/PostgreSQL/Redis steps in
`docs/try_local_system.md` (Ubuntu, WSL2, Windows, macOS, or Nix), run
migrations, then `cargo run`.

## Verify

```bash
curl --head --request GET 'http://localhost:8080/health'   # expect 200 OK
docker compose logs -f hyperswitch-server                    # also: pg, redis-standalone, hyperswitch-control-center
```

## Configuration

- Compose deployments read `config/docker_compose.toml`; if you change it,
  keep `docker-compose.yml` in sync.
- Native runs read `config/development.toml`.
- `admin_api_key` in that file is the admin credential (`test_admin` in
  the sample). Change it for anything beyond local testing.

## Common failures

- "Cannot connect to the Docker/Podman daemon": start Docker/OrbStack.
- "ports are already in use": stop the conflicting services.
- Server never healthy: read `hyperswitch-server` logs, usually a database
  or Redis connection problem.
