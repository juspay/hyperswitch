-- Resolve conflicting payment/payout rows before restoring the old uniqueness rule.
CREATE UNIQUE INDEX CONCURRENTLY blocklist_processor_merchant_id_fingerprint_id_profile_id_index
    ON blocklist (processor_merchant_id, fingerprint_id, profile_id);
