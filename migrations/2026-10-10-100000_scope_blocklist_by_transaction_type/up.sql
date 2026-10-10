-- Existing rows and older writers remain payment-scoped.
ALTER TABLE blocklist
    ADD COLUMN transaction_type "TransactionType" NOT NULL DEFAULT 'payment',
    ADD CONSTRAINT blocklist_transaction_type_check
        CHECK (transaction_type IN ('payment', 'payout'));

-- Build replacements before dropping the old indexes; this migration runs transactionally.
CREATE UNIQUE INDEX blocklist_pm_fingerprint_profile_transaction_type_index
    ON blocklist (processor_merchant_id, fingerprint_id, profile_id, transaction_type);
CREATE UNIQUE INDEX blocklist_pm_fingerprint_null_profile_transaction_type_index
    ON blocklist (processor_merchant_id, fingerprint_id, transaction_type)
    WHERE profile_id IS NULL;
CREATE UNIQUE INDEX blocklist_merchant_fingerprint_profile_transaction_type_index
    ON blocklist (merchant_id, fingerprint_id, profile_id, transaction_type);

DROP INDEX IF EXISTS blocklist_processor_merchant_id_fingerprint_id_profile_id_index;
DROP INDEX IF EXISTS blocklist_pm_fingerprint_null_profile_index;
DROP INDEX IF EXISTS blocklist_merchant_id_fingerprint_id_profile_id_index;
