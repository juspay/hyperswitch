-- This file should undo anything in `up.sql`
ALTER TABLE payment_methods ADD COLUMN IF NOT EXISTS accepted_currency "Currency"[];

ALTER TABLE payment_methods ADD COLUMN IF NOT EXISTS scheme VARCHAR(32);

ALTER TABLE payment_methods ADD COLUMN IF NOT EXISTS token VARCHAR(128);

ALTER TABLE payment_methods ADD COLUMN IF NOT EXISTS cardholder_name VARCHAR(255);

ALTER TABLE payment_methods ADD COLUMN IF NOT EXISTS issuer_name VARCHAR(64);

ALTER TABLE payment_methods ADD COLUMN IF NOT EXISTS issuer_country VARCHAR(64);

ALTER TABLE payment_methods ADD COLUMN IF NOT EXISTS is_stored BOOLEAN;

ALTER TABLE payment_methods ADD COLUMN IF NOT EXISTS direct_debit_token VARCHAR(128);

ALTER TABLE payment_methods ADD COLUMN IF NOT EXISTS swift_code VARCHAR(32);

ALTER TABLE payment_methods ADD COLUMN IF NOT EXISTS payment_method_issuer VARCHAR(128);

CREATE TYPE "PaymentMethodIssuerCode" AS ENUM (
    'jp_hdfc',
    'jp_icici',
    'jp_googlepay',
    'jp_applepay',
    'jp_phonepe',
    'jp_wechat',
    'jp_sofort',
    'jp_giropay',
    'jp_sepa',
    'jp_bacs'
);

ALTER TABLE payment_methods ADD COLUMN IF NOT EXISTS payment_method_issuer_code "PaymentMethodIssuerCode";

ALTER TABLE payment_methods DROP COLUMN IF EXISTS locker_fingerprint_id;

ALTER TABLE payment_methods ADD COLUMN IF NOT EXISTS payment_method_id VARCHAR(64);
UPDATE payment_methods SET payment_method_id = id;
ALTER TABLE payment_methods DROP CONSTRAINT IF EXISTS payment_methods_pkey;
ALTER TABLE payment_methods ADD CONSTRAINT payment_methods_pkey PRIMARY KEY (payment_method_id);
ALTER TABLE payment_methods DROP COLUMN IF EXISTS id;
ALTER TABLE payment_methods ADD COLUMN IF NOT EXISTS id SERIAL;