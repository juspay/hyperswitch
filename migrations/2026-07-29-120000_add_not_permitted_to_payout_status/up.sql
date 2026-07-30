-- Add the terminal `not_permitted` payout status (e.g. Verification-of-Payee
-- refusal). Distinct from the non-terminal `ineligible`.
ALTER TYPE "PayoutStatus" ADD VALUE IF NOT EXISTS 'not_permitted';
