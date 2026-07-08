-- This file should undo anything in `up.sql`
-- Remove the `error_code_based` enum label. Safe only if no rows use it.
DELETE FROM pg_enum
WHERE enumlabel = 'error_code_based'
AND enumtypid = (
  SELECT oid FROM pg_type WHERE typname = 'RevenueRecoveryAlgorithmType'
);
