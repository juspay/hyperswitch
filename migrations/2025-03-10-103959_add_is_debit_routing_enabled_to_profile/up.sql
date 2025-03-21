-- Your SQL goes here
ALTER TABLE business_profile
ADD COLUMN IF NOT EXISTS is_debit_routing_enabled BOOLEAN NOT NULL DEFAULT FALSE;