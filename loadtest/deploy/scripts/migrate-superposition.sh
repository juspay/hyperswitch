#!/usr/bin/env bash
set -euo pipefail

SUPERPOSITION_REPO="${SUPERPOSITION_REPO:?SUPERPOSITION_REPO is required}"
HYPERSWITCH_REPO="${HYPERSWITCH_REPO:?HYPERSWITCH_REPO is required}"
SUPERPOSITION_ORG_ID="${SUPERPOSITION_ORG_ID:?SUPERPOSITION_ORG_ID is required}"
SUPERPOSITION_WORKSPACE_ID="${SUPERPOSITION_WORKSPACE_ID:?SUPERPOSITION_WORKSPACE_ID is required}"
MIGRATIONS_DIR="$(mktemp -d)"
trap 'rm -rf "$MIGRATIONS_DIR"' EXIT

for identifier in "$SUPERPOSITION_ORG_ID" "$SUPERPOSITION_WORKSPACE_ID"; do
  [[ "$identifier" =~ ^[a-zA-Z][a-zA-Z0-9_]*$ ]] || {
    echo "Invalid Superposition identifier: $identifier" >&2
    exit 2
  }
done

cp -a "$SUPERPOSITION_REPO/crates/superposition_types/migrations/." "$MIGRATIONS_DIR/"
sed -i 's/{replaceme}/public/g' \
  "$MIGRATIONS_DIR/2025-10-28-070717_functions-modifications/up.sql"

nix develop "$HYPERSWITCH_REPO" -c diesel migration run \
  --migration-dir "$MIGRATIONS_DIR" \
  --config-file "$SUPERPOSITION_REPO/crates/superposition_types/src/database/diesel.toml"

workspace_schema="${SUPERPOSITION_ORG_ID}_${SUPERPOSITION_WORKSPACE_ID}"
workspace_exists="$(psql "$DATABASE_URL" -Atv ON_ERROR_STOP=1 \
  -c "SELECT EXISTS (
    SELECT 1 FROM superposition.workspaces
    WHERE organisation_id = '$SUPERPOSITION_ORG_ID'
      AND workspace_name = '$SUPERPOSITION_WORKSPACE_ID'
  )")"

if [[ "$workspace_exists" != "t" ]]; then
  {
    printf 'BEGIN;\n'
    printf "INSERT INTO superposition.organisations
      (id, name, created_by, admin_email, updated_by)
      VALUES ('%s', '%s', 'loadtest@local', 'loadtest@local', 'loadtest@local')
      ON CONFLICT (id) DO NOTHING;\n" "$SUPERPOSITION_ORG_ID" "$SUPERPOSITION_ORG_ID"
    sed "s/{replaceme}/$workspace_schema/g" "$SUPERPOSITION_REPO/workspace_template.sql"
    printf "INSERT INTO superposition.workspaces
      (organisation_id, organisation_name, workspace_name, workspace_schema_name,
       workspace_status, workspace_admin_email, created_by, last_modified_by)
      VALUES ('%s', '%s', '%s', '%s', 'ENABLED',
              'loadtest@local', 'loadtest@local', 'loadtest@local');\n" \
      "$SUPERPOSITION_ORG_ID" "$SUPERPOSITION_ORG_ID" \
      "$SUPERPOSITION_WORKSPACE_ID" "$workspace_schema"
    printf 'COMMIT;\n'
  } | psql "$DATABASE_URL" -v ON_ERROR_STOP=1
fi
