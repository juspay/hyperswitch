-- Remove the column `request_overcapture` from the `payment_intent` table
ALTER TABLE payment_intent DROP COLUMN request_overcapture;

-- Remove the column `always_request_overcapture` from the `business_profile` table
ALTER TABLE business_profile DROP COLUMN always_request_overcapture;

-- Remove the column `overcapture_applied` from the `payment_attempt` table
ALTER TABLE payment_attempt DROP COLUMN overcapture_applied;