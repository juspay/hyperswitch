-- Your SQL goes here
-- Add the `error_code_based` variant to the revenue recovery retry algorithm enum.
-- When a profile is set to this, the concrete strategy for the next attempt is
-- decided by Superposition based on the previous attempt's error code.
ALTER TYPE "RevenueRecoveryAlgorithmType" ADD VALUE IF NOT EXISTS 'error_code_based';
