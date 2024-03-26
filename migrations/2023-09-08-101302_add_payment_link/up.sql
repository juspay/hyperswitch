-- Your SQL goes here
CREATE TABLE payment_link (
    payment_link_id VARCHAR(255) NOT NULL,
    payment_id VARCHAR(64) NOT NULL,
    link_to_pay VARCHAR(255) NOT NULL,
    merchant_id VARCHAR(64) NOT NULL,
    amount INT8 NOT NULL,
    currency "Currency",
    created_at TIMESTAMP NOT NULL,
    last_modified_at TIMESTAMP NOT NULL,
    fulfilment_time TIMESTAMP,
    PRIMARY KEY (payment_link_id)
);
