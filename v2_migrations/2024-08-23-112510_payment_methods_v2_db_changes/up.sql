-- Your SQL goes here
ALTER TABLE payment_methods DROP COLUMN IF EXISTS accepted_currency;

ALTER TABLE payment_methods DROP COLUMN IF EXISTS scheme;

ALTER TABLE payment_methods DROP COLUMN IF EXISTS token;

ALTER TABLE payment_methods DROP COLUMN IF EXISTS cardholder_name;

ALTER TABLE payment_methods DROP COLUMN IF EXISTS issuer_name;

ALTER TABLE payment_methods DROP COLUMN IF EXISTS issuer_country;

ALTER TABLE payment_methods DROP COLUMN IF EXISTS payer_country;

ALTER TABLE payment_methods DROP COLUMN IF EXISTS is_stored;

ALTER TABLE payment_methods DROP COLUMN IF EXISTS direct_debit_token;

ALTER TABLE payment_methods DROP COLUMN IF EXISTS swift_code;

ALTER TABLE payment_methods DROP COLUMN IF EXISTS payment_method_issuer;

ALTER TABLE payment_methods DROP COLUMN IF EXISTS payment_method_issuer_code;

DROP TYPE IF EXISTS "PaymentMethodIssuerCode";

ALTER TABLE payment_methods ADD COLUMN IF NOT EXISTS locker_fingerprint_id VARCHAR(64);

ALTER TABLE payment_methods DROP COLUMN IF EXISTS id;
ALTER TABLE payment_methods ADD COLUMN IF NOT EXISTS id VARCHAR(64);
UPDATE payment_methods SET id = payment_method_id;
ALTER TABLE payment_methods DROP CONSTRAINT IF EXISTS payment_methods_pkey;
ALTER TABLE payment_methods ADD CONSTRAINT payment_methods_pkey PRIMARY KEY (id);
ALTER TABLE payment_methods DROP COLUMN IF EXISTS payment_method_id;