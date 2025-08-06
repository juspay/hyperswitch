-- Your SQL goes here
CREATE TYPE "RevenueRecoveryAlgorithmType" AS ENUM ('monitoring', 'smart', 'cascading');

ALTER TABLE business_profile
ADD COLUMN IF NOT EXISTS revenue_recovery_retry_algorithm_type "RevenueRecoveryAlgorithmType",
ADD COLUMN IF NOT EXISTS revenue_recovery_retry_algorithm_data JSONB;
