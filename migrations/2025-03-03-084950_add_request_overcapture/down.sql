-- This file should undo anything in `up.sql`
ALTER TABLE business_profile DROP COLUMN IF EXISTS always_request_overcapture;
ALTER TABLE payment_attempt
DROP COLUMN IF EXISTS request_overcapture,
DROP COLUMN IF EXISTS overcapture_status;
ALTER TABLE payment_intent DROP COLUMN IF EXISTS request_overcapture;