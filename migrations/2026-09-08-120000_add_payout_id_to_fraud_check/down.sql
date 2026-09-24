-- This fails when payout fraud_check rows with a NULL payment_id
-- still exist. Such rows must be deleted before rolling back this migration.
ALTER TABLE fraud_check
    ALTER COLUMN payment_id SET NOT NULL;

ALTER TABLE fraud_check
    DROP CONSTRAINT fraud_check_pkey;

ALTER TABLE fraud_check ADD PRIMARY KEY (frm_id, attempt_id, payment_id, merchant_id);

-- Restore the historical schema exactly, including its redundant unique index.
CREATE UNIQUE INDEX frm_id_index ON fraud_check (frm_id, attempt_id, payment_id, merchant_id);

ALTER TABLE fraud_check
    DROP COLUMN payout_id;
