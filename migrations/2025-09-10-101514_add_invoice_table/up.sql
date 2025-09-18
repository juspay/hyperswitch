CREATE TABLE invoice (
    id VARCHAR(255) PRIMARY KEY,
    subscription_id VARCHAR(255) NOT NULL,
    connector_subscription_id VARCHAR(255),
    merchant_id VARCHAR(255) NOT NULL,
    profile_id VARCHAR(255) NOT NULL,
    merchant_connector_id VARCHAR(255) NOT NULL,
    payment_intent_id VARCHAR(255) UNIQUE,
    payment_method_id VARCHAR(128),
    customer_id VARCHAR(255) NOT NULL,
    amount BIGINT NOT NULL,
    currency "Currency",
    status VARCHAR(50) NOT NULL,
    provider_name VARCHAR(100) NOT NULL,
    metadata JSONB,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    modified_at TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_payment_intent_id ON invoice (payment_intent_id);
CREATE INDEX idx_subscription_id ON invoice (subscription_id);
CREATE INDEX idx_merchant_customer ON invoice (merchant_id, customer_id);
