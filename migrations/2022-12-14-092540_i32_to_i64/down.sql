-- This file should undo anything in `up.sql`
ALTER TABLE mandate
    ALTER COLUMN mandate_amount TYPE integer,
    ALTER COLUMN amount_captured TYPE integer;

ALTER TABLE payment_attempt
    ALTER COLUMN amount TYPE integer,
    ALTER COLUMN offer_amount TYPE integer,
    ALTER COLUMN surcharge_amount TYPE integer,
    ALTER COLUMN tax_amount TYPE integer,
    ALTER COLUMN amount_to_capture TYPE integer;

ALTER TABLE payment_intent
    ALTER COLUMN amount TYPE integer,
    ALTER COLUMN amount_captured TYPE integer;

ALTER TABLE refund
    ALTER COLUMN total_amount TYPE integer,
    ALTER COLUMN refund_amount TYPE integer;
