-- This file should undo anything in `up.sql`
DROP INDEX CONCURRENTLY IF EXISTS payment_intent_processor_merchant_id_payment_id_index;
