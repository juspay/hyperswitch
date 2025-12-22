-- Your SQL goes here

-- This migration backfills the organization_id column in the payouts table.
-- It sets organization_id based on the corresponding merchant_account entry for cases where the organization_id was NULL.
-- This is required for older payout records created before organization_id was introduced as a column in the payouts table.

UPDATE payouts p
SET organization_id = ma.organization_id
FROM merchant_account ma
WHERE p.merchant_id = ma.merchant_id
  AND p.organization_id IS NULL;
