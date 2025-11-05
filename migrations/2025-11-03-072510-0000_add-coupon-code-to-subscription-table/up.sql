-- Your SQL goes here
ALTER TABLE subscription
ADD COLUMN IF NOT EXISTS coupon_codes TEXT[] DEFAULT NULL;