-- Your SQL goes here

CREATE TABLE blocklist (
  id SERIAL PRIMARY KEY,
  merchant_id VARCHAR(64) NOT NULL,
  fingerprint_id VARCHAR(64) NOT NULL,
  data_kind "BlocklistDataKind" NOT NULL,
  metadata JSONB,
  created_at TIMESTAMP NOT NULL
);

CREATE UNIQUE INDEX blocklist_unique_fingerprint_id_index ON blocklist (merchant_id, fingerprint_id);
CREATE INDEX blocklist_merchant_id_data_kind_created_at_index ON blocklist (merchant_id, data_kind, created_at DESC);
