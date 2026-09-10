#! /usr/bin/env bash
# Local Spanner loop: Postgres baseline -> schema dump -> Spanner DDL -> emulator.
# No GCP account, no image build. End to end in a couple of minutes.
#
#   ./scripts/spanner/run_local.sh
#
# Verified end to end on 2026-09-10: 49 tables and 81 indexes apply with zero
# errors, and a payment_intent + payment_attempt row round-trips.

set -euo pipefail
cd "$(dirname "$0")/../.."

PG_CONTAINER="${PG_CONTAINER:-hs-spanner-pg}"
PG_PORT="${PG_PORT:-5434}"
PG_URL="postgres://db_user:db_pass@localhost:${PG_PORT}/hyperswitch_db"
SPANNER_URL="${SPANNER_URL:-postgresql://localhost:5433/hyperswitch_db}"
WORK="${WORK:-/tmp/hs-spanner}"
mkdir -p "$WORK"

step() { printf '\n\033[1m==> %s\033[0m\n' "$1"; }

step "1/6  Postgres baseline (source of truth for the schema)"
if ! docker ps --format '{{.Names}}' | grep -qx "$PG_CONTAINER"; then
    docker rm -f "$PG_CONTAINER" >/dev/null 2>&1 || true
    docker run -d --name "$PG_CONTAINER" \
        -e POSTGRES_USER=db_user -e POSTGRES_PASSWORD=db_pass \
        -e POSTGRES_DB=hyperswitch_db -p "${PG_PORT}:5432" postgres:16 >/dev/null
    until psql "$PG_URL" -c 'select 1' >/dev/null 2>&1; do sleep 1; done
fi
diesel migration run --database-url "$PG_URL"

step "2/6  Dump the end-state schema"
# pg_dump runs *inside* the container: a v15 client refuses a v16 server, and
# Homebrew's psql is usually a major version behind the postgres:16 image.
docker exec "$PG_CONTAINER" pg_dump --schema-only --no-owner --no-acl \
    -U db_user -d hyperswitch_db > "$WORK/pg_schema.sql"
echo "    $(grep -c '^CREATE TABLE' "$WORK/pg_schema.sql") tables, $(grep -c '^CREATE TYPE' "$WORK/pg_schema.sql") enum types"

step "3/6  Translate to Spanner PG dialect"
./scripts/spanner/pg_to_spanner_ddl.py "$WORK/pg_schema.sql" \
    > "$WORK/spanner.sql" 2> "$WORK/report.txt"
echo "    $(grep -c '^CREATE TABLE' "$WORK/spanner.sql") tables, $(grep -cE '^CREATE (UNIQUE )?INDEX' "$WORK/spanner.sql") indexes"
grep -E '^\[' "$WORK/report.txt" | sed 's/^/    /'

step "4/6  Start emulator + PGAdapter (fresh: the emulator is in-memory)"
docker compose -f docker-compose-spanner.yml up -d >/dev/null
docker compose -f docker-compose-spanner.yml restart spanner-emulator pgadapter >/dev/null
until psql "$SPANNER_URL" -tAc 'select 1' >/dev/null 2>&1; do sleep 2; done

step "5/6  Probe the dialect"
psql "$SPANNER_URL" -v ON_ERROR_STOP=0 -f scripts/spanner/probe.sql \
    > "$WORK/probe.log" 2>&1
echo "    $(grep -c ERROR "$WORK/probe.log") errors (4 expected - the NOT SUPPORTED block)"

step "6/6  Apply the baseline"
# PGAdapter batches consecutive DDL and hands the batch to Spanner as a unit, so
# one bad statement fails everything after it. The generated file is pure DDL for
# exactly this reason - a stray `--` comment inside the batch fails the parse.
psql "$SPANNER_URL" -v ON_ERROR_STOP=0 -f "$WORK/spanner.sql" > "$WORK/apply.log" 2>&1
errors=$(grep -c ERROR "$WORK/apply.log" || true)
tables=$(psql "$SPANNER_URL" -tAc "select count(*) from information_schema.tables where table_schema='public'")
idx=$(psql "$SPANNER_URL" -tAc "select count(*) from information_schema.indexes where table_schema='public' and index_type<>'PRIMARY_KEY'")
echo "    $tables tables, $idx indexes, $errors errors"

if [[ "$errors" -ne 0 ]]; then
    echo
    echo "Failures in $WORK/apply.log:"
    grep ERROR "$WORK/apply.log" | sed 's/ - Statement.*//' | sort -u | head
    exit 1
fi

cat <<EOF

Schema is live on the emulator. Read $WORK/report.txt for what was translated.

Run the router against it (no docker build):
  RUST_MIN_STACK=16777216 cargo run --bin router --features spanner
EOF
