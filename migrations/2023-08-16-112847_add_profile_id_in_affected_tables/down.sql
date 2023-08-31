ALTER TABLE payment_intent DROP COLUMN IF EXISTS profile_id;

ALTER TABLE merchant_connector_account DROP COLUMN IF EXISTS profile_id;

ALTER TABLE merchant_account DROP COLUMN IF EXISTS default_profile;

ALTER TABLE refund DROP COLUMN IF EXISTS profile_id;

ALTER TABLE dispute DROP COLUMN IF EXISTS profile_id;

ALTER TABLE payout_attempt DROP COLUMN IF EXISTS profile_id;
