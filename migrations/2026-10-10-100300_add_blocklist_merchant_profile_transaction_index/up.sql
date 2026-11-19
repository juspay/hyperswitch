-- Build the replacement before removing the existing uniqueness indexes.
CREATE UNIQUE INDEX CONCURRENTLY blocklist_merchant_fingerprint_profile_transaction_type_index
    ON blocklist (merchant_id, fingerprint_id, profile_id, transaction_type);
