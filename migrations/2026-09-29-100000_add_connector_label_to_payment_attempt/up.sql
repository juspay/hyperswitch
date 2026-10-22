ALTER TABLE payment_attempt
    ADD COLUMN IF NOT EXISTS connector_label VARCHAR(255);
