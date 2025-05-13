-- This file should undo anything in `up.sql`
ALTER TABLE routing_algorithm
DROP COLUMN IF EXISTS decision_engine_routing_id;
