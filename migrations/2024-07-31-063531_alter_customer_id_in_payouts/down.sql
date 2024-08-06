ALTER TABLE payouts
ALTER COLUMN customer_id
SET
    NOT NULL,
ALTER COLUMN address_id
SET
    NOT NULL;

ALTER TABLE payout_attempt
ALTER COLUMN customer_id
SET
    NOT NULL,
ALTER COLUMN address_id
SET
    NOT NULL;