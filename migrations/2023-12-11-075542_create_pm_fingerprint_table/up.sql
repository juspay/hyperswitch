-- Your SQL goes here

CREATE TABLE pm_fingerprint (
  fingerprint_id VARCHAR(64) PRIMARY KEY,
  kms_hash TEXT NOT NULL
);

CREATE INDEX pm_fingerprint_fingerprint_id_kms_hash ON pm_fingerprint (fingerprint_id, kms_hash DESC);
