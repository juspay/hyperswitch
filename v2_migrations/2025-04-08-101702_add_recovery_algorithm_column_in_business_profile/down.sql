-- This file should undo anything in `up.sql`
ALTER TABLE business_profile DROP COLUMN IF EXISTS recovery_retry_algorithm;

DROP TYPE IF EXISTS "RecoveryAlgorithm";
