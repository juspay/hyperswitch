-- Your SQL goes here
CREATE TABLE IF NOT EXISTS merchant_acquirer (
    merchant_acquirer_id VARCHAR(64) NOT NULL,
    acquirer_assigned_merchant_id VARCHAR(64) NOT NULL,
    merchant_name VARCHAR(255) NOT NULL,
    mcc VARCHAR(64) NOT NULL,
    merchant_country_code VARCHAR(64) NOT NULL,
    network VARCHAR(64) NOT NULL,
    acquirer_bin VARCHAR(64) NOT NULL,
    acquirer_ica VARCHAR(64),
    acquirer_fraud_rate FLOAT NOT NULL,
    profile_id VARCHAR(64) NOT NULL,
    created_at TIMESTAMP NOT NULL,
    last_modified_at TIMESTAMP NOT NULL,
    PRIMARY KEY (merchant_acquirer_id)
);
