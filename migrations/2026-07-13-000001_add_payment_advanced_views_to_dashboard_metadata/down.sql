-- Postgres does not support removing a value from an enum type.
-- This migration is intentionally a no-op on rollback; the enum value
-- 'payment_advanced_views' remains but is simply unused.
SELECT 1;
