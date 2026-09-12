ALTER TABLE payment_attempt
    ADD COLUMN IF NOT EXISTS active_frm_id VARCHAR(64);

-- Existing fraud-check records are associated at the payment level. Associate
-- every historical attempt for that payment with the same FRM decision before
-- payment_id becomes nullable on fraud_check.
UPDATE payment_attempt AS pa
SET active_frm_id = fc.frm_id
FROM fraud_check AS fc
WHERE pa.payment_id = fc.payment_id
  AND pa.merchant_id = fc.merchant_id
  AND pa.active_frm_id IS NULL;
