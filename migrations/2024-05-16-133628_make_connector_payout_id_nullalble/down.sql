-- This file should undo anything in `up.sql`
UPDATE payout_attempt
SET connector_payout_id = ''
WHERE connector_payout_id IS NULL;

ALTER TABLE payout_attempt
ALTER COLUMN connector_payout_id SET NOT NULL;