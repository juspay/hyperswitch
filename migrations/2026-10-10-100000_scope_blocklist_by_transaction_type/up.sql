-- Existing rows are payment-scoped
ALTER TABLE blocklist ADD COLUMN transaction_type TEXT;

UPDATE blocklist SET transaction_type = 'payment' WHERE transaction_type IS NULL;

-- Build replacements concurrently before dropping the old indexes.
CREATE UNIQUE INDEX CONCURRENTLY blocklist_pm_fingerprint_profile_transaction_type_index
    ON blocklist (processor_merchant_id, fingerprint_id, profile_id, transaction_type);
CREATE UNIQUE INDEX CONCURRENTLY blocklist_pm_fingerprint_null_profile_transaction_type_index
    ON blocklist (processor_merchant_id, fingerprint_id, transaction_type)
    WHERE profile_id IS NULL;
CREATE UNIQUE INDEX CONCURRENTLY blocklist_merchant_fingerprint_profile_transaction_type_index
    ON blocklist (merchant_id, fingerprint_id, profile_id, transaction_type);

DROP INDEX CONCURRENTLY IF EXISTS blocklist_processor_merchant_id_fingerprint_id_profile_id_index;
DROP INDEX CONCURRENTLY IF EXISTS blocklist_pm_fingerprint_null_profile_index;
DROP INDEX CONCURRENTLY IF EXISTS blocklist_merchant_id_fingerprint_id_profile_id_index;
