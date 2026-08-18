-- Prepares payment_methods for the index swap in the following migration, which makes
-- (id, locker_fingerprint_id) unique and allows an id to span several rows.
--
-- Both statements are expected to affect zero rows on a healthy database. They exist so the
-- CONCURRENTLY index builds that follow cannot meet a violation: a concurrent unique build that
-- fails leaves an INVALID index behind, and recovering from that is a manual DROP INDEX.

-- 1. Redaction used to stamp a fixed sentinel over the fingerprint. That string cannot survive a
--    unique index on (id, locker_fingerprint_id) once an id can carry more than one retired row,
--    so redaction now writes NULL and the existing sentinels are cleared to match.
UPDATE payment_methods
SET locker_fingerprint_id = NULL
WHERE locker_fingerprint_id = 'FINGERPRINT_ID_REDACTED';

-- 2. Retire every row but the newest under any id that already has more than one, using the same
--    ordering the application uses to resolve an id to its current row.
--
--    `id IS NOT NULL` is load-bearing, not defensive. On this track payment_methods.id is nullable
--    -- rows predating the v2 backfill have no id -- and PARTITION BY would gather every one of
--    them into a single partition and retire all but one. A NULL id names no payment method and
--    is exempt from both new indexes anyway, since Postgres treats NULLs as distinct.
--
--    Nothing is deleted: superseded rows are marked redacted and keep their vault entry.
UPDATE payment_methods p
SET status = 'redacted',
    locker_fingerprint_id = NULL,
    last_modified = NOW()
FROM (
    SELECT ctid,
           ROW_NUMBER() OVER (
               PARTITION BY id
               ORDER BY created_at DESC, locker_fingerprint_id DESC NULLS LAST
           ) AS row_rank
    FROM payment_methods
    WHERE id IS NOT NULL
) ranked
WHERE p.ctid = ranked.ctid
  AND ranked.row_rank > 1;
