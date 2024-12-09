-- This file should undo anything in `up.sql`
ALTER TABLE refund DROP COLUMN IF EXISTS split_refunds;
