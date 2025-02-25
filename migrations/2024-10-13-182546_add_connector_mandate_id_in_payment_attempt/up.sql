-- Your SQL goes here
ALTER TABLE
    payment_attempt
ADD
    COLUMN connector_mandate_detail JSONB DEFAULT NULL;