-- This file should undo anything in `up.sql`
ALTER TABLE payouts DROP COLUMN IF EXISTS payout_link_id;