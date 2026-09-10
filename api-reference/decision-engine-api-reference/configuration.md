---
title: "Configuration"
description: "Configure Decision Engine via config files and environment variable overrides."
---

# Configuration Guide

This document explains how to configure Decision Engine for local and on-prem deployments.

## Primary Config Files

- `config/development.toml`: used for host/source runs
- `config/docker-configuration.toml`: used for Docker and Compose runs
- `helm-charts/config/development.toml`: Kubernetes chart template config

These files already exist with all required sections. Edit the one that matches your runtime rather than copying from `config.example.toml`, which is incomplete.

## Config Sections

### Server

```toml
[server]
host = "0.0.0.0"
port = 8080
```

`host` is the bind address. Use `0.0.0.0` for Docker/deployed, `127.0.0.1` for local-only.

### Logging

```toml
[log.console]
enabled = true
level = "DEBUG"
log_format = "default"
```

`log_format` accepts `"default"` (human-readable) or `"json"` (structured, recommended for prod).

### Metrics

```toml
[metrics]
host = "0.0.0.0"
port = 9094
```

Prometheus metrics are exposed at `host:port/metrics`. Used by the `monitoring` Compose profile (Prometheus scrapes `9094`, Grafana at `3000`).

### Rate Limiting

```toml
[limit]
request_count = 1
duration = 60
```

Controls rate limiting on the delete APIs. `request_count` requests allowed per `duration` seconds.

### Redis cache config

```toml
[cache_config]
service_config_redis_prefix = "DE_service_config_"
service_config_ttl = 300    # Redis TTL for service config entries, in seconds
```

### Database

MySQL and PostgreSQL use separate config sections.

**MySQL:**

```toml
[database]
username = "db_user"
password = "db_pass"
host = "localhost"
port = 3306
dbname = "decision_engine_db"
```

**PostgreSQL:**

```toml
[pg_database]
pg_username = "db_user"
pg_password = "db_pass"
pg_host = "localhost"
pg_port = 5432
pg_dbname = "decision_engine_db"
```

For Docker Compose runs both are pre-wired via service names in `config/docker-configuration.toml`.

### Multi-Tenant Schema

```toml
[tenant_secrets]
public = { schema = "public" }
```

Maps tenant identifiers to database schemas. The shipped config files (`config/development.toml`, `config/docker-configuration.toml`) define only the `public` tenant — add entries for any additional tenant you want to support.

Some routes resolve the tenant from an `x-tenant-id` request header rather than the authenticated merchant, and reject the request outright if it's missing (`TE_03`). Send `x-tenant-id: public` on `GET /health/diagnostics`, every `GET /analytics/*` route, and `POST /gateway-score/reset` — see [API Guide](/decision-engine-api-reference/api-reference/guides/api-ref#environment-setup).

### Redis

```toml
[redis]
host = "127.0.0.1"
port = 6379
```

Redis is required — it's used for caching routing config and service config. For Docker, use the service name as host.

### Auth

```toml
[user_auth]
jwt_secret = "change_me_in_production_use_32chars!!"
jwt_expiry_seconds = 86400
email_verification_enabled = false
```

Use a strong, random `jwt_secret` — 32+ characters recommended. Set `email_verification_enabled = true` if you've wired an email provider.

### Admin Secret

```toml
[admin_secret]
secret = "test_admin"
```

Used to authenticate the `POST /merchant-account/create` endpoint (admin bootstrap). Change this in production.

### API Key Auth

```toml
api_key_auth_enabled = true
```

Top-level flag. When `true`, protected routes accept `x-api-key` in addition to JWT bearer tokens. When `false`, the auth middleware allows **all requests through without authentication** — this effectively disables auth for every protected route. Do not set this to `false` in any environment that should enforce auth.

### Analytics

Both Kafka and ClickHouse require `enabled = true` — without it, analytics is disabled even if the connection details are configured.

```toml
[analytics.kafka]
enabled = true
brokers = "localhost:9092"
api_topic = "api"
domain_topic = "domain"

[analytics.clickhouse]
enabled = true
url = "http://localhost:8123"
user = "decision_engine"
password = "decision_engine"
```

Decision outcomes are published to Kafka and consumed into ClickHouse. Both are required for analytics and audit dashboard views. For Docker runs, these are pre-configured and enabled via the Compose profiles.

### TLS

```toml
[tls]
certificate = "cert.pem"
private_key = "key.pem"
```

Paths to PEM-format certificate and key files. Only needed if you're terminating TLS at the app layer rather than a reverse proxy.

### API Client

```toml
[api_client]
client_idle_timeout = 90
pool_max_idle_per_host = 10
identity = ""
```

Controls the outbound HTTP client used for upstream calls. Set `identity` to a PEM path if you need mTLS.

## Secrets Management

By default, secrets in config are stored in plaintext. For production, use one of the two supported backends.

### AWS KMS

Requires the `kms-aws` feature (included in the `release` feature set).

```toml
[secrets_management]
secrets_manager = "aws_kms"

[secrets_management.aws_kms]
key_id = "your-kms-key-id"
region = "us-east-1"
```

### HashiCorp Vault

Requires the `kms-hashicorp-vault` feature.

```toml
[secrets_management]
secrets_manager = "hashi_corp_vault"

[secrets_management.hashi_corp_vault]
url = "http://127.0.0.1:8200"
token = "hvs.your_token"
```

When a secrets manager is configured, sensitive fields like `database.password` and `user_auth.jwt_secret` are resolved from the vault rather than the config file.

## Environment Overrides

Selected values can be overridden at runtime via environment variables. This is useful in Helm deployments via `extraEnvVars`. Consult `src/config.rs` for the full mapping.

## Related Docs

- [Installation](/decision-engine-api-reference/installation)
- [Local Setup Guide](/decision-engine-api-reference/local-setup)
- [API Guide](/decision-engine-api-reference/api-reference/guides/api-ref)
- [API Reference](/decision-engine-api-reference/api-reference)
