-- This file should undo anything in `up.sql`
ALTER TABLE dynamic_routing_stats
DROP COLUMN IF EXISTS global_success_based_connector;