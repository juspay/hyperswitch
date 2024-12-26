-- This file should undo anything in `up.sql`
ALTER TABLE dynamic_routing_stats
DROP COLUMN IF EXISTS payment_method_type;
