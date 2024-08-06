-- This file should undo anything in `up.sql`
ALTER TABLE merchant_account
ADD COLUMN merchant_id VARCHAR(64);
