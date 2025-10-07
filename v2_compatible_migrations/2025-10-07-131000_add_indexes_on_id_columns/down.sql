-- Drop indexes on id columns
-- This will remove the indexes created for id column performance optimization

DROP INDEX IF EXISTS customers_id_index;

DROP INDEX IF EXISTS payment_intent_id_index;

DROP INDEX IF EXISTS payment_attempt_id_index;

DROP INDEX IF EXISTS payment_methods_id_index;

DROP INDEX IF EXISTS refund_id_index;
