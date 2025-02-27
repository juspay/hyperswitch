-- Your SQL goes here
ALTER TABLE events ADD COLUMN IF NOT EXISTS is_overall_delivery_successful BOOLEAN;
