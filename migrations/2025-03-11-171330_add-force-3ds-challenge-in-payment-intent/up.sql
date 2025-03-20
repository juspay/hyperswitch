-- Your SQL goes here
ALTER TABLE payment_intent
ADD COLUMN IF NOT EXISTS force_3ds_challenge_overwrite boolean DEFAULT false;