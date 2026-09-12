# Manual migrations

Migrations in this directory are **deliberately excluded** from `migrations/`, so
`diesel migration run` (and therefore a routine deploy) will never apply them.
They need a human to choose the moment and the method.

Run one only when you have read its `up.sql` and decided how to apply it.

## Why the widening migration is here

`2026-09-10-180000_widen_narrow_integers_for_spanner` changes 19 columns from
`smallint`/`integer` to `bigint`. In Postgres that is a **full table rewrite**
holding `ACCESS EXCLUSIVE` for the duration - on `payment_intent` and
`payment_attempt` that is an outage, not a migration.

The application code does **not** require these columns to be `bigint`. Rust
reads a Postgres `integer` into an `i64` without complaint, so the code can ship
first and the schema can follow whenever you are ready. That ordering is
deliberate:

1. Deploy the code (safe, no schema change).
2. Apply the widening later, using `up_online.sql` on large tables.

Spanner is the only backend that *requires* the widening, because its PostgreSQL
dialect rejects `smallint` outright and silently stores `integer` as int64.
