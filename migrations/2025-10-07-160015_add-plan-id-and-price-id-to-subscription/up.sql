-- Your SQL goes here
ALTER TABLE subscription
ADD COLUMN IF NOT EXISTS plan_id varchar(128),
ADD COLUMN IF NOT EXISTS price_id varchar(128);