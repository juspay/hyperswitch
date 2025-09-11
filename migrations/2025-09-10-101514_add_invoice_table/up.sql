CREATE TABLE invoices (
    invoice_id VARCHAR(255) PRIMARY KEY,
    subscription_id VARCHAR(255) NOT NULL,
    connector_subscription_id VARCHAR(255),
    merchant_id VARCHAR(255) NOT NULL,
    profile_id VARCHAR(255) NOT NULL,
    merchant_connector_id VARCHAR(255) NOT NULL,
    payment_intent_id VARCHAR(255) UNIQUE,
    payment_method_id VARCHAR(128),
    customer_id VARCHAR(255) NOT NULL,
    amount BIGINT NOT NULL,
    currency VARCHAR(3) NOT NULL,
    status VARCHAR(50) NOT NULL,
    provider_name VARCHAR(100) NOT NULL,
    metadata JSONB,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    modified_at TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_payment_intent_id ON invoices (payment_intent_id);
CREATE INDEX idx_subscription_id ON invoices (subscription_id);
CREATE INDEX idx_merchant_customer ON invoices (merchant_id, customer_id);
