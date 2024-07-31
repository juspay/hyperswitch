-- This file should undo anything in `up.sql`
ALTER COLUMN merchant_account
ADD COLUMN return_url VARCHAR(255);

ALTER COLUMN merchant_account
ADD COLUMN enable_payment_response_hash BOOLEAN DEFAULT FALSE;
