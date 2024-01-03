-- Your SQL goes here
CREATE TABLE IF NOT EXISTS business_profile (
    profile_id VARCHAR(64) PRIMARY KEY,
    merchant_id VARCHAR(64) NOT NULL,
    profile_name VARCHAR(64) NOT NULL,
    created_at TIMESTAMP NOT NULL,
    modified_at TIMESTAMP NOT NULL,
    return_url TEXT,
    enable_payment_response_hash BOOLEAN NOT NULL DEFAULT TRUE,
    payment_response_hash_key VARCHAR(255) DEFAULT NULL,
    redirect_to_merchant_with_http_post BOOLEAN NOT NULL DEFAULT FALSE,
    webhook_details JSON,
    metadata JSON,
    routing_algorithm JSON,
    intent_fulfillment_time BIGINT,
    frm_routing_algorithm JSONB,
    payout_routing_algorithm JSONB,
    is_recon_enabled BOOLEAN NOT NULL DEFAULT FALSE
);
