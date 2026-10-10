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
ALTER TABLE blocklist DROP COLUMN transaction_type;
