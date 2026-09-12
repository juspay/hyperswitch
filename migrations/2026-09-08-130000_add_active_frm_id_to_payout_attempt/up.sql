ALTER TABLE payout_attempt
    ADD COLUMN IF NOT EXISTS active_frm_id VARCHAR(64);
