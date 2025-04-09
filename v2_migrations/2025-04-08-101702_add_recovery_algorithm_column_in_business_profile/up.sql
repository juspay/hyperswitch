-- Your SQL goes here
CREATE TYPE "RecoveryAlgorithmType" AS ENUM ('monitoring', 'smart', 'cascading');

ALTER TABLE business_profile
ADD COLUMN IF NOT EXISTS recovery_retry_algorithm_type "RecoveryAlgorithmType" DEFAULT 'monitoring';
