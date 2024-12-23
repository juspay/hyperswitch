-- Your SQL goes here
ALTER TABLE dynamic_routing_stats
ADD COLUMN IF NOT EXISTS payment_method_type VARCHAR(64);
