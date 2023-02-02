ALTER TABLE payment_attempt DROP COLUMN IF EXISTS payment_issuer;

ALTER TABLE payment_attempt DROP COLUMN IF EXISTS payment_experience;

DROP TYPE IF EXISTS "PaymentIssuer";

DROP TYPE IF EXISTS "PaymentExperience";
