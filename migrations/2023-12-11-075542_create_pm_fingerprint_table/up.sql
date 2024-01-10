-- Your SQL goes here

CREATE TABLE pm_fingerprint (
  id SERIAL PRIMARY KEY,
  fingerprint_id VARCHAR(64) NOT NULL,
  encrypted_fingerprint TEXT NOT NULL
);

CREATE INDEX pm_fingerprint_fingerprint_id_kms_hash ON pm_fingerprint (fingerprint_id, encrypted_fingerprint DESC);
