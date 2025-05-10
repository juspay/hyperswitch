-- This file should undo anything in `up.sql`
ALTER TABLE routing_algorithm
DROP COLUMN decision_engine_routing_id;
