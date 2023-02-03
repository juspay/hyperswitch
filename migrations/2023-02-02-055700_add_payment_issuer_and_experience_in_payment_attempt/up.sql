-- Your SQL goes here
CREATE TYPE "PaymentIssuer" AS ENUM (
    'klarna',
    'affirm',
    'afterpay_clearpay',
    'american_express',
    'bank_of_america',
    'barclays',
    'capital_one',
    'chase',
    'citi',
    'discover',
    'navy_federal_credit_union',
    'pentagon_federal_credit_union',
    'synchrony_bank',
    'wells_fargo'
);

CREATE TYPE "PaymentExperience" AS ENUM (
    'redirect_to_url',
    'invoke_sdk_client',
    'display_qr_code',
    'one_click',
    'link_wallet',
    'invoke_payment_app'
);

ALTER TABLE payment_attempt
ADD COLUMN IF NOT EXISTS payment_issuer "PaymentIssuer";

ALTER TABLE payment_attempt
ADD COLUMN IF NOT EXISTS payment_experience "PaymentExperience";
