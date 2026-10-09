-- Your SQL goes here
CREATE INDEX CONCURRENTLY IF NOT EXISTS payment_intent_processor_merchant_id_payment_id_index ON payment_intent (processor_merchant_id, payment_id);