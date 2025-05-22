-- Your SQL goes here
ALTER TABLE authentication
ADD COLUMN IF NOT EXISTS psd2_sca_exemption_type "ScaExemptionType" NULL;
