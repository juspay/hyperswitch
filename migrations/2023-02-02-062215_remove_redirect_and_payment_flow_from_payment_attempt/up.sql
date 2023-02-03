ALTER TABLE payment_attempt DROP COLUMN IF EXISTS redirect;

ALTER TABLE payment_attempt DROP COLUMN IF EXISTS payment_flow;

DROP TYPE IF EXISTS "PaymentFlow";
