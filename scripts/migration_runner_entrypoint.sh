#! /usr/bin/env bash

set -eo pipefail

# Check if DATABASE_URL is provided, otherwise construct it from individual components
if [[ -z "${DATABASE_URL:-}" ]]; then
    if [[ -z "${POSTGRES_HOST:-}" || -z "${POSTGRES_USER:-}" || -z "${POSTGRES_PASSWORD:-}" || -z "${POSTGRES_DB:-}" ]]; then
        echo 'Error: Either DATABASE_URL or all of POSTGRES_HOST, POSTGRES_USER, POSTGRES_PASSWORD, POSTGRES_DB must be provided'
        exit 1
    fi
    export DATABASE_URL="postgresql://${POSTGRES_USER}:${POSTGRES_PASSWORD}@${POSTGRES_HOST}:${POSTGRES_PORT:-5432}/${POSTGRES_DB}"
fi

# Run diesel migrations
diesel migration run
