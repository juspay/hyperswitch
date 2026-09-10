-- Widen every narrow integer column to bigint.
--
-- Spanner's PostgreSQL dialect has a single integer width: int8. `smallint` is
-- rejected outright, and `integer` is accepted at DDL time but silently stored
-- as int64 and reported as OID 20 on the wire - so a Rust `i32` field fails at
-- runtime, not at migration time.
--
-- Widening Postgres to match keeps ONE schema shape for both backends: the
-- models become plain `i64` and no feature-gated SQL type shim is needed.
--
-- WARNING: `ALTER COLUMN TYPE` rewrites the table and holds ACCESS EXCLUSIVE
-- for the duration. On the hot tables below this is not safe to run online at
-- scale - use a batched backfill (add column, dual-write, swap) in production.

ALTER TABLE batch_blocklist_jobs ALTER COLUMN total_rows TYPE bigint;
ALTER TABLE batch_blocklist_jobs ALTER COLUMN succeeded_rows TYPE bigint;
ALTER TABLE batch_blocklist_jobs ALTER COLUMN failed_rows TYPE bigint;
ALTER TABLE blocklist_fingerprint ALTER COLUMN id TYPE bigint;
ALTER TABLE blocklist_lookup ALTER COLUMN id TYPE bigint;
ALTER TABLE business_profile ALTER COLUMN max_auto_retries_enabled TYPE bigint;
ALTER TABLE business_profile ALTER COLUMN dispute_polling_interval TYPE bigint;
ALTER TABLE captures ALTER COLUMN capture_sequence TYPE bigint;  -- HOT TABLE: do not run this online at scale
ALTER TABLE configs ALTER COLUMN id TYPE bigint;
ALTER TABLE dashboard_metadata ALTER COLUMN id TYPE bigint;
ALTER TABLE file_metadata ALTER COLUMN file_size TYPE bigint;
ALTER TABLE fraud_check ALTER COLUMN frm_score TYPE bigint;
ALTER TABLE locker_mock_up ALTER COLUMN id TYPE bigint;
ALTER TABLE payment_attempt ALTER COLUMN multiple_capture_count TYPE bigint;  -- HOT TABLE: do not run this online at scale
ALTER TABLE payment_intent ALTER COLUMN attempt_count TYPE bigint;  -- HOT TABLE: do not run this online at scale
ALTER TABLE payment_intent ALTER COLUMN authorization_count TYPE bigint;  -- HOT TABLE: do not run this online at scale
ALTER TABLE payouts ALTER COLUMN attempt_count TYPE bigint;  -- HOT TABLE: do not run this online at scale
ALTER TABLE process_tracker ALTER COLUMN retry_count TYPE bigint;  -- HOT TABLE: do not run this online at scale
ALTER TABLE user_roles ALTER COLUMN id TYPE bigint;
