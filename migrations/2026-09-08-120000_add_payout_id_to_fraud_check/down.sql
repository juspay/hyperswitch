DROP INDEX IF EXISTS fraud_check_payment_id_merchant_id_index;

-- This intentionally fails when payout fraud-check rows with a NULL payment_id
-- still exist. Such rows must be migrated before rolling back this migration.
ALTER TABLE fraud_check
    ALTER COLUMN payment_id SET NOT NULL;

ALTER TABLE fraud_check
    DROP CONSTRAINT fraud_check_pkey;

ALTER TABLE fraud_check ADD PRIMARY KEY (frm_id, attempt_id, payment_id, merchant_id);

-- Restore the historical schema exactly, including its redundant unique index.
CREATE UNIQUE INDEX frm_id_index
    ON fraud_check (frm_id, attempt_id, payment_id, merchant_id);

ALTER TABLE fraud_check
    DROP COLUMN payout_id;
