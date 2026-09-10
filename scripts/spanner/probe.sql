-- Dialect probe for Spanner's PostgreSQL dialect, via PGAdapter.
--
--   psql "postgresql://localhost:5433/hyperswitch_db" -f scripts/spanner/probe.sql
--
-- Every statement mirrors something hyperswitch's schema or query layer actually
-- does. Results below were verified against the emulator with PGAdapter 0.55.3
-- on 2026-09-10; re-run after any version bump, since these are the facts the
-- spanner_types.rs and router_derive shims are built on.

SET spanner.support_drop_cascade = true;
DROP TABLE IF EXISTS probe_types;

-- ---------------------------------------------------------------- SUPPORTED
CREATE TABLE probe_types (
    id          varchar(64) NOT NULL,
    status      varchar(64),          -- enums flattened to varchar (46 PG types)
    secret      bytea,                -- 45 encrypted columns
    meta        jsonb,                -- 95 jsonb + 17 json columns
    created_at  timestamptz DEFAULT CURRENT_TIMESTAMP,
    amount      bigint,
    flag        bool,
    tags        text[],
    items       jsonb[],              -- VERIFIED OK: arrays of jsonb round-trip
    PRIMARY KEY (id)
);
CREATE INDEX probe_status_idx ON probe_types (status, created_at);

INSERT INTO probe_types (id, status, secret, meta, amount, flag, tags, items)
VALUES ('probe_1', 'charged', '\x0102ff'::bytea,
        '{"connector":"stripe","nested":{"n":1}}'::jsonb,
        1999, true, ARRAY['a','b'], ARRAY['{"sku":"a"}'::jsonb]);

-- Extended query protocol with bound parameters — how diesel talks. VERIFIED OK.
PREPARE probe_sel AS
    SELECT id, status, secret, meta, created_at, amount, flag, tags, items
    FROM probe_types WHERE id = $1 AND status = $2;
EXECUTE probe_sel('probe_1', 'charged');

-- The one ON CONFLICT in the codebase (diesel_models/src/query/blocklist.rs:24).
-- VERIFIED OK.
INSERT INTO probe_types (id, status) VALUES ('probe_1', 'ignored')
ON CONFLICT (id) DO NOTHING;

-- RETURNING backs every generic op in diesel_models/src/query/generics.rs.
-- VERIFIED OK.
UPDATE probe_types SET amount = 2999 WHERE id = 'probe_1'
RETURNING id, amount, created_at;

-- jsonb operators used by metadata filters. VERIFIED OK.
SELECT meta->>'connector', meta @> '{"connector":"stripe"}'::jsonb FROM probe_types;

DROP TABLE probe_types;

-- -------------------------------------------------------------- NOT SUPPORTED
-- Each of these is expected to ERROR. If one starts succeeding, the dialect has
-- moved and the corresponding shim can be retired.
--
--   timestamp without time zone -> ERROR: Type <timestamp> is not supported.
--   smallint                    -> ERROR: use bigint or int8 instead.
--   json                        -> ERROR: use jsonb instead.
--   CREATE TYPE .. AS ENUM      -> ERROR: Statement is not supported.
--   pg_typeof()                 -> ERROR: Postgres function is not supported.
--   expression indexes          -> ERROR: Expressions are not supported.
--   partial indexes             -> only `col IS NOT NULL` conjunctions.
--
-- And the trap that is NOT an error: `integer` and `serial` are ACCEPTED at DDL
-- time but silently stored as int64 and reported as OID 20 on the wire. A
-- diesel column typed Int4/Int2 therefore decodes garbage from a column the
-- DDL claimed was `integer`. That is why the narrow columns are widened to
-- bigint on both sides instead of shimmed -- see the migration in
-- migrations_manual/ and the Int8 columns in diesel_models/src/schema.rs.
--
-- Why `diesel migration run` cannot be pointed at Spanner at all. Verified
-- 2026-09-10: it fails before migration 0001, on diesel's own ledger table.
--
--   CREATE TABLE IF NOT EXISTS __diesel_schema_migrations (
--       version VARCHAR(50) PRIMARY KEY NOT NULL,
--       run_on TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP)
--     -> ERROR: Type <timestamp> is not supported.
--
-- and with run_on repaired to timestamptz, the name itself is rejected:
--
--     -> ERROR: Table name not valid: __diesel_schema_migrations.
--
-- Diesel has nowhere to record which migrations ran, so the runner cannot work
-- here no matter what the migrations contain. For the record, the first two
-- would fail anyway:
--
--   CREATE FUNCTION ... LANGUAGE plpgsql  (0000_diesel_initial_setup)
--     -> ERROR: Unsupported option value 'plpgsql' for 'language'.
--   CREATE TYPE "AuthenticationType" AS ENUM (2022-09-29_create_initial_tables)
--     -> ERROR: Statement is not supported.
CREATE TABLE probe_reject (id varchar(8) PRIMARY KEY, a timestamp without time zone);
CREATE TABLE probe_reject2 (id varchar(8) PRIMARY KEY, a smallint);
CREATE TABLE probe_reject3 (id varchar(8) PRIMARY KEY, a json);
CREATE TYPE probe_enum AS ENUM ('a','b');
