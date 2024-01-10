-- Your SQL goes here

CREATE TABLE pm_blocklist (
  id SERIAL PRIMARY KEY,
  merchant_id VARCHAR(64) NOT NULL,
  fingerprint TEXT NOT NULL,
  fingerprint_type VARCHAR(64) NOT NULL,
  metadata TEXT
);

CREATE INDEX pm_blocklist_merchant_id_pm_hash ON pm_blocklist (merchant_id, fingerprint DESC);
