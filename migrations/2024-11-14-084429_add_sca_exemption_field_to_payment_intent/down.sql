-- This file should undo anything in `up.sql`
ALTER TABLE payment_intent DROP COLUMN sca_exemption_required;

DROP TYPE "ScaExemptionType";