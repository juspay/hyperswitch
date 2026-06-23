-- This file should undo anything in `up.sql`
ALTER TABLE routing_algorithm DROP COLUMN IF EXISTS processor_merchant_id;
ALTER TABLE routing_algorithm DROP COLUMN IF EXISTS created_by;
