#!/usr/bin/env sh
set -eu

export PGPASSWORD=docker
database=config
source=/loadtest/kronos-workspace.sql

schemas=$(psql -h 127.0.0.1 -U postgres -d "$database" -Atc \
  "SELECT workspace_schema_name FROM superposition.workspaces ORDER BY workspace_schema_name")

for schema in $schemas; do
  echo "superposition: applying Kronos workspace migration to $schema"
  sed 's/{p}/kronos_/g' "$source" | \
    PGOPTIONS="-c search_path=$schema,public" \
    psql -v ON_ERROR_STOP=1 -h 127.0.0.1 -U postgres -d "$database"
done
