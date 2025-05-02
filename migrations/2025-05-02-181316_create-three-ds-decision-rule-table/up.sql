-- Your SQL goes here
CREATE TABLE three_ds_decision_rule (
  id VARCHAR(64) PRIMARY KEY NOT NULL,
  rule JSONB NOT NULL,
  name VARCHAR(255) NOT NULL,
  description TEXT,
  active BOOLEAN NOT NULL,
  created_at TIMESTAMP NOT NULL DEFAULT now()::TIMESTAMP,
  modified_at TIMESTAMP NOT NULL DEFAULT now()::TIMESTAMP
);
