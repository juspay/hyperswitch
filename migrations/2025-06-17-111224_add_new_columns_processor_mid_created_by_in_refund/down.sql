-- This file should undo anything in `up.sql`
ALTER TABLE refund DROP COLUMN IF EXISTS processor_merchant_id;
ALTER TABLE refund DROP COLUMN IF EXISTS created_by;