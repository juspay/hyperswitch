-- This file should undo anything in `up.sql`
ALTER TABLE authentication
DROP COLUMN IF EXISTS psd2_sca_exemption_type;
