-- Add payout support to fraud_check while preserving the existing payment flow.
ALTER TABLE fraud_check
    ADD COLUMN IF NOT EXISTS payout_id VARCHAR(64);

-- payment_id is part of the existing primary key. Replace the primary key
-- before making payment_id nullable.
ALTER TABLE fraud_check
    DROP CONSTRAINT fraud_check_pkey;

ALTER TABLE fraud_check ADD PRIMARY KEY (frm_id, merchant_id);

ALTER TABLE fraud_check
    ALTER COLUMN payment_id DROP NOT NULL;

-- This index duplicates the original primary key index and is not useful.
DROP INDEX IF EXISTS frm_id_index;
