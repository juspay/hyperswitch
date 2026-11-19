-- This file should undo anything in `up.sql`
CREATE UNIQUE INDEX CONCURRENTLY IF NOT EXISTS payment_attempt_payment_id_merchant_id_attempt_id_index ON payment_attempt (payment_id, merchant_id, attempt_id);
