-- This file should undo anything in `up.sql`
-- Remove profile_id from payment_attempt table
ALTER TABLE payment_attempt
DROP COLUMN IF EXISTS profile_id;

-- Remove organization_id from payment_attempt table
ALTER TABLE payment_attempt
DROP COLUMN IF EXISTS organization_id;

-- Remove organization_id from payment_intent table
ALTER TABLE payment_intent
DROP COLUMN IF EXISTS organization_id;

-- Remove organization_id from refund table
ALTER TABLE refund
DROP COLUMN IF EXISTS organization_id;

-- Remove organization_id from dispute table
ALTER TABLE dispute
DROP COLUMN IF EXISTS organization_id;