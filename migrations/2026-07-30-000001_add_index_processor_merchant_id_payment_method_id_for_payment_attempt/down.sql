-- This file should undo anything in `up.sql`
-- DROP INDEX CONCURRENTLY also needs to run outside a transaction.
DROP INDEX CONCURRENTLY IF EXISTS payment_attempt_processor_merchant_id_payment_method_id_index;
