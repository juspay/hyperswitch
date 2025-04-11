-- Your SQL goes here
CREATE TYPE "RevenueRecoveryAlgorithmType" AS ENUM ('monitoring', 'smart', 'cascading');

ALTER TABLE business_profile
ADD COLUMN IF NOT EXISTS revenue_recovery_retry_algorithm_type "RevenueRecoveryAlgorithmType" DEFAULT 'monitoring';nue_recovery_retry_algorithm_type VARCHAR DEFAULT 'monitoring';

