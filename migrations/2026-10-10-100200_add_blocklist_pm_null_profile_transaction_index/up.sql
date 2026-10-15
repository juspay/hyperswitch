-- Build the replacement before removing the existing uniqueness indexes.
CREATE UNIQUE INDEX CONCURRENTLY blocklist_pm_fingerprint_null_profile_transaction_type_index
    ON blocklist (processor_merchant_id, fingerprint_id, transaction_type)
    WHERE profile_id IS NULL;
