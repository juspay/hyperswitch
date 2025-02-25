-- Your SQL goes here
ALTER TABLE payment_methods
    DROP COLUMN IF EXISTS accepted_currency,
    DROP COLUMN IF EXISTS scheme,
    DROP COLUMN IF EXISTS token,
    DROP COLUMN IF EXISTS cardholder_name,
    DROP COLUMN IF EXISTS issuer_name,
    DROP COLUMN IF EXISTS issuer_country,
    DROP COLUMN IF EXISTS payer_country,
    DROP COLUMN IF EXISTS is_stored,
    DROP COLUMN IF EXISTS direct_debit_token,
    DROP COLUMN IF EXISTS swift_code,
    DROP COLUMN IF EXISTS payment_method_issuer,
    DROP COLUMN IF EXISTS payment_method_issuer_code,
    DROP COLUMN IF EXISTS metadata,
    DROP COLUMN IF EXISTS payment_method,
    DROP COLUMN IF EXISTS payment_method_type;

DROP TYPE IF EXISTS "PaymentMethodIssuerCode";

ALTER TABLE payment_methods
    ADD COLUMN IF NOT EXISTS locker_fingerprint_id VARCHAR(64),
    ADD COLUMN IF NOT EXISTS payment_method_type_v2 VARCHAR(64),
    ADD COLUMN IF NOT EXISTS payment_method_subtype VARCHAR(64);

ALTER TABLE payment_methods DROP COLUMN IF EXISTS id;
ALTER TABLE payment_methods ADD COLUMN IF NOT EXISTS id VARCHAR(64);
UPDATE payment_methods SET id = payment_method_id;
ALTER TABLE payment_methods DROP CONSTRAINT IF EXISTS payment_methods_pkey;
ALTER TABLE payment_methods ADD CONSTRAINT payment_methods_pkey PRIMARY KEY (id);
ALTER TABLE payment_methods DROP COLUMN IF EXISTS payment_method_id;
