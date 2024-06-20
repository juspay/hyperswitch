-- This file should undo anything in `up.sql`
ALTER TABLE users DROP COLUMN totp_status;
ALTER TABLE users DROP COLUMN totp_secret;
ALTER TABLE users DROP COLUMN totp_recovery_codes;

DROP TYPE "TotpStatus";
