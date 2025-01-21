-- Your SQL goes here
ALTER TABLE dynamic_routing_stats
ADD COLUMN IF NOT EXISTS global_success_based_connector VARCHAR(64);