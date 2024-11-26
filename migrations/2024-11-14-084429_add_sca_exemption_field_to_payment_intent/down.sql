-- This file should undo anything in `up.sql`
ALTER TABLE payment_intent DROP COLUMN psd2_sca_exemption_type;

DROP TYPE "ScaExemptionType";