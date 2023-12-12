-- Your SQL goes here

CREATE TABLE pm_blocklist (
  merchant_id VARCHAR(64) PRIMARY KEY,
  pm_hash TEXT NOT NULL
);

CREATE INDEX pm_blocklist_merchant_id_pm_hash ON pm_blocklist (merchant_id, pm_hash DESC);
