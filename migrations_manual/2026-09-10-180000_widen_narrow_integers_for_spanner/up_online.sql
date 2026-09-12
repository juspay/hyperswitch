-- Online widening for the five hot tables. Template - run one table at a time,
-- and verify row counts between steps.
--
-- `up.sql` (plain ALTER COLUMN TYPE) is correct but takes ACCESS EXCLUSIVE for a
-- full table rewrite. Use it for dev, CI, and the small tables. Use this for
-- payment_intent, payment_attempt, payouts, captures and process_tracker.
--
-- Repeat the block below per column. Shown for payment_intent.attempt_count.

-- 1. Add the wide column. Instant: a nullable column with no default does not
--    rewrite the table.
ALTER TABLE payment_intent ADD COLUMN attempt_count_new bigint;

-- 2. Keep it in step with writes. Drop this trigger in step 5.
CREATE OR REPLACE FUNCTION payment_intent_attempt_count_sync()
RETURNS trigger AS $$
BEGIN
    NEW.attempt_count_new := NEW.attempt_count;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER payment_intent_attempt_count_sync_trg
    BEFORE INSERT OR UPDATE ON payment_intent
    FOR EACH ROW EXECUTE FUNCTION payment_intent_attempt_count_sync();

-- 3. Backfill in batches. Run repeatedly until it reports 0 rows; keep the batch
--    small enough that each statement stays well under any statement timeout.
--    Do NOT do this as one UPDATE - that takes the lock you are avoiding.
UPDATE payment_intent
SET attempt_count_new = attempt_count
WHERE payment_id IN (
    SELECT payment_id FROM payment_intent
    WHERE attempt_count_new IS NULL
    LIMIT 10000
);

-- 4. Verify before swapping. Must return 0.
SELECT count(*) FROM payment_intent
WHERE attempt_count_new IS DISTINCT FROM attempt_count;

-- 5. Swap. This does take a brief ACCESS EXCLUSIVE, but only for a rename -
--    milliseconds, not a rewrite. Take it inside an explicit transaction with a
--    short lock_timeout so it fails fast rather than queueing behind traffic.
BEGIN;
SET LOCAL lock_timeout = '3s';
DROP TRIGGER payment_intent_attempt_count_sync_trg ON payment_intent;
DROP FUNCTION payment_intent_attempt_count_sync();
ALTER TABLE payment_intent DROP COLUMN attempt_count;
ALTER TABLE payment_intent RENAME COLUMN attempt_count_new TO attempt_count;
ALTER TABLE payment_intent ALTER COLUMN attempt_count SET NOT NULL;
COMMIT;

-- 6. Re-create any index that referenced the old column.
