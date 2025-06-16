--- Your SQL goes here
CREATE TYPE "SuccessBasedRoutingConclusiveState" AS ENUM(
  'true_positive',
  'false_positive',
  'true_negative',
  'false_negative'
);

CREATE TABLE IF NOT EXISTS dynamic_routing_stats (
    payment_id VARCHAR(64) NOT NULL,
    attempt_id VARCHAR(64) NOT NULL,
    merchant_id VARCHAR(64) NOT NULL,
    profile_id VARCHAR(64) NOT NULL,
    amount BIGINT NOT NULL,
    success_based_routing_connector VARCHAR(64) NOT NULL,
    payment_connector VARCHAR(64) NOT NULL,
    currency "Currency",
    payment_method VARCHAR(64),
    capture_method "CaptureMethod",
    authentication_type "AuthenticationType",
    payment_status "AttemptStatus" NOT NULL,
    conclusive_classification "SuccessBasedRoutingConclusiveState" NOT NULL,
    created_at TIMESTAMP NOT NULL,
    PRIMARY KEY(attempt_id, merchant_id)
);
CREATE INDEX profile_id_index ON dynamic_routing_stats (profile_id);
