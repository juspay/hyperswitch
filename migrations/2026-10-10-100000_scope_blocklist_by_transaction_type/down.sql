-- Refuse rollback when removing flow scoping would collapse entries. Never delete twins.
-- Keep writes out between the check, old-index restoration, and column removal.
LOCK TABLE blocklist IN ACCESS EXCLUSIVE MODE;
DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM blocklist
        GROUP BY merchant_id, fingerprint_id, profile_id
        HAVING COUNT(DISTINCT transaction_type) > 1
    ) OR EXISTS (
        SELECT 1 FROM blocklist
        WHERE processor_merchant_id IS NOT NULL
        GROUP BY processor_merchant_id, fingerprint_id, profile_id
        HAVING COUNT(DISTINCT transaction_type) > 1
    ) THEN
        RAISE EXCEPTION 'Cannot remove blocklist transaction scoping while payment/payout twins exist';
    END IF;
END $$;

-- Restore every old uniqueness rule first. Any conflicting rows abort the transaction intact.
CREATE UNIQUE INDEX blocklist_processor_merchant_id_fingerprint_id_profile_id_index
    ON blocklist (processor_merchant_id, fingerprint_id, profile_id);
CREATE UNIQUE INDEX blocklist_pm_fingerprint_null_profile_index
    ON blocklist (processor_merchant_id, fingerprint_id)
    WHERE profile_id IS NULL;
CREATE UNIQUE INDEX blocklist_merchant_id_fingerprint_id_profile_id_index
    ON blocklist (merchant_id, fingerprint_id, profile_id);

DROP INDEX blocklist_pm_fingerprint_profile_transaction_type_index;
DROP INDEX blocklist_pm_fingerprint_null_profile_transaction_type_index;
DROP INDEX blocklist_merchant_fingerprint_profile_transaction_type_index;
ALTER TABLE blocklist
    DROP CONSTRAINT blocklist_transaction_type_check,
    DROP COLUMN transaction_type;
