#!/usr/bin/env bash
# Stops the stack started by up.sh. Add -v to also delete the database volume: bash scripts/down.sh -v
set -u
export PATH="$PATH:/c/Program Files/Docker/Docker/resources/bin"
cd "$(dirname "$0")/../.."
docker compose -f docker-compose.yml -f cost-router/docker/compose.override.yml down "$@"
