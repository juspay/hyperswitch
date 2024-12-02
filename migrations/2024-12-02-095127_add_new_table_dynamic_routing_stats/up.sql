--- Your SQL goes here
CREATE TYPE "ConclusiveClassification" AS ENUM(
  'true_positive',
  'false_positive',
  'true_negative',
  'false_negative'
);

CREATE TABLE IF NOT EXISTS dynamic_routing_stats (
    payment_id VARCHAR(255) NOT NULL,
    tenant_id VARCHAR(64) NOT NULL,
    merchant_id VARCHAR(255) NOT NULL,
    profile_id VARCHAR(255) NOT NULL,
    success_based_routing_connector VARCHAR(255),
    payment_connector VARCHAR(255),
    currency VARCHAR(255),
    payment_method VARCHAR(255),
    capture_method VARCHAR(255),
    authentication_type VARCHAR(255),
    payment_status VARCHAR(255),
    conclusive_classification "ConclusiveClassification",
    created_at TIMESTAMP NOT NULL,
    modified_at TIMESTAMP NOT NULL,
    PRIMARY KEY(payment_id)
);

CREATE UNIQUE INDEX IF NOT EXISTS payment_id_index ON dynamic_routing_stats (payment_id);
