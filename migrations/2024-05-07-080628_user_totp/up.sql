-- Your SQL goes here
CREATE TYPE "TotpStatus" AS ENUM (
  'set',
  'in_progress',
  'not_set'
);

ALTER TABLE users ADD COLUMN IF NOT EXISTS totp_status "TotpStatus" DEFAULT 'not_set' NOT NULL;
ALTER TABLE users ADD COLUMN IF NOT EXISTS totp_secret bytea DEFAULT NULL;
ALTER TABLE users ADD COLUMN IF NOT EXISTS totp_recovery_codes TEXT[] DEFAULT NULL;
