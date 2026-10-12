-- Add the terminal `not_permitted` payout status (e.g. Verification-of-Payee
-- refusal). Distinct from the non-terminal `ineligible`.
ALTER TYPE "PayoutStatus" ADD VALUE IF NOT EXISTS 'not_permitted';

-- Outgoing webhook event for the above status. Emitted instead of
-- `payout_failed` so the refusal is explicit to merchants.
ALTER TYPE "EventType" ADD VALUE IF NOT EXISTS 'payout_not_permitted';
