-- Your SQL goes here
ALTER TABLE payout_attempt ALTER COLUMN connector_payout_id DROP NOT NULL;

UPDATE payout_attempt SET connector_payout_id = NULL WHERE connector_payout_id = '';