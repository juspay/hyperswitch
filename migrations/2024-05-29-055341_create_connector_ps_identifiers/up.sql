CREATE TABLE connector_ps_identifiers (
    id SERIAL,
    merchant_id VARCHAR(64) NOT NULL,
    mca_id VARCHAR(64) NOT NULL,
    connect_account_id VARCHAR(64) NOT NULL,
    customer_id VARCHAR(64) NOT NULL,
    pm_id VARCHAR(64) NOT NULL,
    customer_ps_id VARCHAR(64) NOT NULL,
    pm_ps_id VARCHAR(64),
    created_at TIMESTAMP NOT NULL DEFAULT now()::TIMESTAMP,
    modified_at TIMESTAMP NOT NULL DEFAULT now()::TIMESTAMP,
    PRIMARY KEY (id)
);