-- This file should undo anything in `up.sql`
ALTER TABLE payout_attempt DROP COLUMN IF EXISTS additional_payout_method_data;