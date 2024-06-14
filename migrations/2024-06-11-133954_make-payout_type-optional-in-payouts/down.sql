-- This file should undo anything in `up.sql`
ALTER TABLE payouts ALTER COLUMN payout_type SET NOT NULL;