-- Your SQL goes here

CREATE TABLE blocklist_lookup (
  id SERIAL PRIMARY KEY,
  merchant_id VARCHAR(64) NOT NULL,
  fingerprint TEXT NOT NULL
);

CREATE UNIQUE INDEX blocklist_lookup_merchant_id_fingerprint_index ON blocklist_lookup (merchant_id, fingerprint);
