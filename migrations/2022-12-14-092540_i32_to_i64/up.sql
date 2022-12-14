-- Your SQL goes here
ALTER TABLE mandate
    ALTER COLUMN mandate_amount TYPE bigint,
    ALTER COLUMN amount_captured TYPE bigint;

ALTER TABLE payment_attempt
    ALTER COLUMN amount TYPE bigint,
    ALTER COLUMN offer_amount TYPE bigint,
    ALTER COLUMN surcharge_amount TYPE bigint,
    ALTER COLUMN tax_amount TYPE bigint,
    ALTER COLUMN amount_to_capture TYPE bigint;

ALTER TABLE payment_intent
    ALTER COLUMN amount TYPE bigint,
    ALTER COLUMN amount_captured TYPE bigint;

ALTER TABLE refund
    ALTER COLUMN total_amount TYPE bigint,
    ALTER COLUMN refund_amount TYPE bigint;
