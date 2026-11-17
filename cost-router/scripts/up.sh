#!/usr/bin/env bash
# Brings up the minimal Hyperswitch stack (pg, redis, migrations, superposition, router) with the prebuilt image.
# Uses only NEW files under cost-router/docker (trimmed config copy + compose override); no tracked file is touched.
set -u
export PATH="$PATH:/c/Program Files/Docker/Docker/resources/bin"
cd "$(dirname "$0")/../.."
DC="docker compose -f docker-compose.yml -f cost-router/docker/compose.override.yml"
$DC up -d pg redis-standalone
# migration_runner ends with "Failed to apply patch" (schema.rs patch, unrelated); migrations themselves apply.
$DC up migration_runner || true
$DC up -d superposition
$DC up superposition-init          # ~3 min, seeds 912 overrides
$DC up -d --no-deps --force-recreate hyperswitch-server
for i in $(seq 1 40); do curl -sf localhost:8080/health && break; sleep 3; done
