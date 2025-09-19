CREATE TABLE invoice (
    id VARCHAR(64) PRIMARY KEY,
    subscription_id VARCHAR(128) NOT NULL,
    connector_subscription_id VARCHAR(128),
    merchant_id VARCHAR(64) NOT NULL,
    profile_id VARCHAR(64) NOT NULL,
    merchant_connector_id VARCHAR(128) NOT NULL,
    payment_intent_id VARCHAR(64) UNIQUE,
    payment_method_id VARCHAR(64),
    customer_id VARCHAR(64) NOT NULL,
    amount BIGINT NOT NULL,
    currency "Currency" NOT NULL,
    status VARCHAR(64) NOT NULL,
    provider_name VARCHAR(128) NOT NULL,
    metadata JSONB,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    modified_at TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_subscription_id ON invoice (subscription_id);
