---
title: "MySQL Setup"
description: "Set up Decision Engine with a MySQL database."
---

# MySQL Setup Guide

This page provides MySQL-focused commands. For the full end-to-end setup — CLI, Docker, Compose, Helm — see the [Local Setup Guide](/decision-engine-api-reference/local-setup).

## Docker Compose (GHCR track)

```bash
export DECISION_ENGINE_TAG=v1.4
COMPOSE_PROFILES= docker compose --profile mysql-ghcr up -d
```

With dashboard + docs:

```bash
COMPOSE_PROFILES= docker compose --profile dashboard-mysql-ghcr up -d
```

## Docker Compose (Local build track)

```bash
COMPOSE_PROFILES= docker compose --profile mysql-local up -d --build
```

With dashboard + docs:

```bash
COMPOSE_PROFILES= docker compose --profile dashboard-mysql-local up -d --build
```

## Make targets

```bash
make init-mysql-ghcr
make init-mysql-local
```

## Verify

```bash
curl http://localhost:8080/health
```

Expected response:

```json
{"message":"Health is good"}
```

## Related Docs

- [Installation](/decision-engine-api-reference/installation)
- [Local Setup Guide](/decision-engine-api-reference/local-setup)
- [PostgreSQL Setup](/decision-engine-api-reference/setup-guide-postgres)
- [Configuration](/decision-engine-api-reference/configuration)
