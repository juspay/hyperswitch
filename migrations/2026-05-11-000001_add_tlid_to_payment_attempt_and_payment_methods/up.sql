ALTER TABLE payment_attempt
    ADD COLUMN IF NOT EXISTS network_transaction_link_id VARCHAR(255);

ALTER TABLE payment_methods
    ADD COLUMN IF NOT EXISTS network_transaction_link_id VARCHAR(255);
