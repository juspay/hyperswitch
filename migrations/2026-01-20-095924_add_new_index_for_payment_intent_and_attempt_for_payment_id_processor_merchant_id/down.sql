-- This file should undo anything in `up.sql`
DROP INDEX IF EXISTS payment_intent_processor_merchant_id_payment_id_index;

DROP INDEX IF EXISTS payment_attempt_processor_merchant_id_payment_id_index;
