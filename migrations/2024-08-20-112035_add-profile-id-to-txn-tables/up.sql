-- Your SQL goes here
ALTER TABLE payment_attempt
ADD COLUMN IF NOT EXISTS profile_id VARCHAR(64) NOT NULL DEFAULT 'default_profile';

-- Add organization_id to payment_attempt table
ALTER TABLE payment_attempt
ADD COLUMN IF NOT EXISTS organization_id VARCHAR(32) NOT NULL DEFAULT 'default_org';

-- Add organization_id to payment_intent table
ALTER TABLE payment_intent
ADD COLUMN IF NOT EXISTS organization_id VARCHAR(32) NOT NULL DEFAULT 'default_org';

-- Add organization_id to refund table
ALTER TABLE refund
ADD COLUMN IF NOT EXISTS organization_id VARCHAR(32) NOT NULL DEFAULT 'default_org';

-- Add organization_id to dispute table
ALTER TABLE dispute
ADD COLUMN IF NOT EXISTS organization_id VARCHAR(32) NOT NULL DEFAULT 'default_org';

-- This doesn't work on V2
-- The below backfill step has to be run after the code deployment
-- UPDATE payment_attempt pa
-- SET organization_id = ma.organization_id
-- FROM merchant_account ma
-- WHERE pa.merchant_id = ma.merchant_id;

-- UPDATE payment_intent pi
-- SET organization_id = ma.organization_id
-- FROM merchant_account ma
-- WHERE pi.merchant_id = ma.merchant_id;

-- UPDATE refund r
-- SET organization_id = ma.organization_id
-- FROM merchant_account ma
-- WHERE r.merchant_id = ma.merchant_id;

-- UPDATE payment_attempt pa
-- SET profile_id = pi.profile_id
-- FROM payment_intent pi
-- WHERE pa.payment_id = pi.payment_id
--   AND pa.merchant_id = pi.merchant_id
--   AND pi.profile_id IS NOT NULL;
