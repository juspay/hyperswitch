UPDATE payment_methods
SET locker_fingerprint_id = NULL
WHERE locker_fingerprint_id = 'FINGERPRINT_ID_REDACTED';

-- `id IS NOT NULL` is load bearing: PARTITION BY gathers every NULL id into one partition, and
-- rows with a NULL id are exempt from the new indexes anyway.
UPDATE payment_methods p
SET status = 'redacted',
    locker_fingerprint_id = NULL,
    last_modified = NOW()
FROM (
    SELECT ctid,
           ROW_NUMBER() OVER (
               PARTITION BY id
               ORDER BY (status <> 'redacted') DESC, created_at DESC
           ) AS row_rank
    FROM payment_methods
    WHERE id IS NOT NULL
) ranked
WHERE p.ctid = ranked.ctid
  AND ranked.row_rank > 1;
