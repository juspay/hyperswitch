ALTER TABLE payment_attempt
    ADD COLUMN IF NOT EXISTS active_frm_id VARCHAR(64);

ALTER TABLE payout_attempt
    ADD COLUMN IF NOT EXISTS active_frm_id VARCHAR(64);

-- Existing fraud_check records are associated at the payment level. Backfill
-- every attempt for that payment with the frm_id.
UPDATE payment_attempt
SET active_frm_id = fraud_check.frm_id
FROM fraud_check
WHERE payment_attempt.payment_id = fraud_check.payment_id
  AND payment_attempt.merchant_id = fraud_check.merchant_id
  AND payment_attempt.active_frm_id IS NULL;
