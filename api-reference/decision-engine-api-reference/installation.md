---
title: "Installation"
description: "Everything needed to get Decision Engine running locally, end to end."
---

# Installation Guide

This section covers everything needed to get Decision Engine running locally — from a single `docker compose up` to the full CLI, Docker, Compose, and Helm matrix.

## Prerequisites

The Quick Start below pulls prebuilt images from GHCR, so you do **not** need Rust, `make`, or a local database. You do need:

- **Docker Engine 20+**
- **Docker Compose v2+** (the `docker compose` subcommand, not the legacy `docker-compose` binary)
- **The repository cloned locally** — the compose command reads `docker-compose.yaml`, so run it from the repo root:

  ```bash
  git clone https://github.com/juspay/decision-engine.git
  cd decision-engine
  ```

- Network access to pull `ghcr.io/juspay/...` images and a few GB of free disk (the first run pulls the app, PostgreSQL, Redis, Kafka, ClickHouse, and Mailpit).

## Quick Start

The fastest path to a running instance. Every service in `docker-compose.yaml` is gated behind a profile, so a profile is required — there is no default/unprofiled bring-up:

```bash
docker compose --profile postgres-ghcr up -d
curl http://localhost:8080/health
```

Expected response:

```json
{ "message": "Health is good" }
```

For the API, dashboard, and docs together, use `--profile dashboard-postgres-ghcr` instead — see [Dashboard](/decision-engine-api-reference/dashboard-guide).

## In This Section

| Page | Use it for |
| --- | --- |
| [Local Setup](/decision-engine-api-reference/local-setup) | The canonical guide — Compose profiles, source builds, Docker images without Compose, Helm, and troubleshooting. |
| [PostgreSQL Setup](/decision-engine-api-reference/setup-guide-postgres) | Postgres-specific Compose profiles, `make` targets, and verification. |
| [MySQL Setup](/decision-engine-api-reference/setup-guide-mysql) | The same, for MySQL. |
| [Configuration](/decision-engine-api-reference/configuration) | Config file reference and environment variable overrides once the service is up. |
| [Dashboard](/decision-engine-api-reference/dashboard-guide) | Bring up the React operator dashboard alongside the API. |

## Choosing A Database

Decision Engine supports PostgreSQL and MySQL as interchangeable backends. Pick one and follow its dedicated guide, or go straight to [Local Setup](/decision-engine-api-reference/local-setup) if you want the full profile matrix (dashboard, monitoring, source builds) rather than a database-first walkthrough.

## Next Steps

- [API Guide](/decision-engine-api-reference/api-reference/guides/api-ref) — copy-paste `curl` examples once the service is running.
