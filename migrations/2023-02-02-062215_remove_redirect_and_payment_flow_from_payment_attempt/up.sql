ALTER TABLE payment_attempt DROP IF EXISTS redirect;

ALTER TABLE payment_attempt DROP IF EXISTS payment_flow;

DROP TYPE IF EXISTS "PaymentFlow";
