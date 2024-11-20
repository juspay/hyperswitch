CREATE TYPE "ScaExemptionType" AS ENUM (
    'low_value',
    'transaction_risk_analysis'
);

ALTER TABLE payment_intent ADD COLUMN IF NOT EXISTS sca_exemption_required "ScaExemptionType";