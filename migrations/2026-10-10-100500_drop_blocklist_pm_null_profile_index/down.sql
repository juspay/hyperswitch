-- Resolve conflicting payment/payout rows before restoring the old uniqueness rule.
CREATE UNIQUE INDEX CONCURRENTLY blocklist_pm_fingerprint_null_profile_index
    ON blocklist (processor_merchant_id, fingerprint_id)
    WHERE profile_id IS NULL;
