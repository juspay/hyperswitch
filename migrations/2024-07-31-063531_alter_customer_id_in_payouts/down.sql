-- ALTER TABLE payouts
-- ALTER COLUMN customer_id
-- SET
--     NOT NULL,
-- ALTER COLUMN address_id
-- SET
--     NOT NULL;

-- Below query will add the columns to the end, which would require diesel_models changes
-- ALTER TABLE payout_attempt
-- ADD COLUMN IF NOT EXISTS customer_id VARCHAR(64),
-- ADD COLUMN IF NOT EXISTS address_id VARCHAR(64);

SELECT 1;