CREATE TYPE "ScaExemptionType" AS ENUM (
    'low_value_exemption',
    'low_risk_exemption',
    'secure_corporate_exemption',
    'trusted_beneficiary_exemption',
    'merchant_initiated_transaction_exemption'
);

ALTER TABLE payment_intent ADD COLUMN IF NOT EXISTS sca_exemption_required "ScaExemptionType";