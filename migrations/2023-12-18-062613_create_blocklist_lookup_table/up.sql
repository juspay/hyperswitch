-- Your SQL goes here

CREATE TABLE blocklist_lookup (
  id SERIAL PRIMARY KEY,
  merchant_id VARCHAR(64) NOT NULL,
  kms_decrypted_hash TEXT NOT NULL
);

CREATE INDEX blocklist_lookup_merchant_id_kms_decrypted_hash ON blocklist_lookup (merchant_id, kms_decrypted_hash DESC);
