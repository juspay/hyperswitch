-- This file should undo anything in `up.sql`
ALTER COLUMN merchant_account
ADD COLUMN return_url VARCHAR(255);
