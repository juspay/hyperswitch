-- This file should undo anything in `up.sql`
CREATE UNIQUE INDEX CONCURRENTLY IF NOT EXISTS payment_intent_payment_id_merchant_id_index ON payment_intent (payment_id, merchant_id);
