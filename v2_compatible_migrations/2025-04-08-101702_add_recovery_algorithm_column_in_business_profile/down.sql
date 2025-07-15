-- This file should undo anything in `up.sql`
ALTER TABLE business_profile 
DROP COLUMN IF EXISTS revenue_recovery_retry_algorithm_type,
DROP COLUMN IF EXISTS revenue_recovery_retry_algorithm_data;

DROP TYPE IF EXISTS "RevenueRecoveryAlgorithmType";
