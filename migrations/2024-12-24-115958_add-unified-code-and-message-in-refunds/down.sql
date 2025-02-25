-- This file should undo anything in `up.sql`
ALTER TABLE refund DROP COLUMN IF EXISTS unified_code;
ALTER TABLE refund DROP COLUMN IF EXISTS unified_message;