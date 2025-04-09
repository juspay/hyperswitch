-- Your SQL goes here
CREATE TYPE "RecoveryAlgorithm" AS ENUM ('monitoring', 'smart', 'cascading');

ALTER TABLE business_profile
ADD COLUMN IF NOT EXISTS recovery_retry_algorithm "RecoveryAlgorithm" DEFAULT 'monitoring';
