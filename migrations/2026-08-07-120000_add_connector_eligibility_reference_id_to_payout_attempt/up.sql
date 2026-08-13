-- Connector's reference for the payout eligibility check (e.g. Deutsche Bank's
-- Verification-of-Payee id). Kept separate from `connector_payout_id`, which is
-- only set when the payee is eligible and is later overwritten by the transfer,
-- so the check stays traceable for reconciliation on every outcome.
ALTER TABLE payout_attempt
    ADD COLUMN IF NOT EXISTS connector_eligibility_reference_id VARCHAR(128);
