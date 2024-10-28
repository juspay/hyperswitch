-- This file should undo anything in `up.sql`
ALTER TABLE users ADD COLUMN preferred_merchant_id VARCHAR(64);
