-- This file should undo anything in `up.sql`
ALTER TABLE payouts
DROP COLUMN IF EXISTS organization_id;