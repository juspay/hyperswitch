-- Remove the column `enable_overcapture` from the `payment_intent` table
ALTER TABLE payment_intent DROP COLUMN enable_overcapture;

-- Remove the column `always_enable_overcapture` from the `business_profile` table
ALTER TABLE business_profile DROP COLUMN always_enable_overcapture;

-- Remove the column `is_overcapture_enabled` from the `payment_attempt` table
ALTER TABLE payment_attempt DROP COLUMN is_overcapture_enabled;