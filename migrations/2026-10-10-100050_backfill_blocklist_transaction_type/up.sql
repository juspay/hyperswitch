-- Existing rows are payment-scoped. The preceding migration commits the
-- column addition first, releasing its ALTER TABLE lock before this backfill.
-- Keep this file as one DO statement; metadata.toml allows per-batch commits.
DO $$
DECLARE
    rows_updated BIGINT;
BEGIN
    LOOP
        WITH batch AS (
            SELECT ctid
            FROM blocklist
            WHERE transaction_type IS NULL
            LIMIT 5000
            FOR UPDATE
        )
        UPDATE blocklist AS b
        SET transaction_type = 'payment'
        FROM batch
        WHERE b.ctid = batch.ctid;

        GET DIAGNOSTICS rows_updated = ROW_COUNT;

        COMMIT;

        RAISE NOTICE 'Updated % rows', rows_updated;
        EXIT WHEN rows_updated = 0;

        PERFORM pg_sleep(1);
    END LOOP;
END;
$$;
