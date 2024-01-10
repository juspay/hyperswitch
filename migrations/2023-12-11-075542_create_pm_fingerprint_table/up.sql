-- Your SQL goes here

CREATE TYPE "BlocklistDataKind" AS ENUM (
    'payment_method',
    'card_bin',
    'extended_card_bin'
);

CREATE TABLE blocklist_fingerprint (
  id SERIAL PRIMARY KEY,
  merchant_id VARCHAR(64) NOT NULL,
  fingerprint_id VARCHAR(64) NOT NULL,
  data_kind "BlocklistDataKind" NOT NULL,
  encrypted_fingerprint TEXT NOT NULL,
  created_at TIMESTAMP NOT NULL
);

CREATE UNIQUE INDEX blocklist_fingerprint_merchant_id_fingerprint_id_index
ON blocklist_fingerprint (merchant_id, fingerprint_id);
