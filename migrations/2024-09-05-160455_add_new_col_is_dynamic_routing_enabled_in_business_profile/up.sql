-- Your SQL goes here
ALTER TABLE business_profile
ADD COLUMN is_dynamic_routing_enabled BOOLEAN DEFAULT FALSE;
