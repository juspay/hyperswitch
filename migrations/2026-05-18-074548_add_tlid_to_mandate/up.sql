ALTER TABLE mandate
    ADD COLUMN IF NOT EXISTS network_transaction_link_id VARCHAR(255);
