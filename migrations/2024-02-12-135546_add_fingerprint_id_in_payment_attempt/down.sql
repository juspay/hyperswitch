-- This file should undo anything in `up.sql`
ALTER TABLE payment_attempt ADD COLUMN IF NOT EXISTS fingerprint_id VARCHAR(64);
