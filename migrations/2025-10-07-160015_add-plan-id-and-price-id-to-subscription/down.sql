-- This file should undo anything in `up.sql`
ALTER TABLE subscription DROP COLUMN IF EXISTS plan_id;
ALTER TABLE subscription DROP COLUMN IF EXISTS price_id;