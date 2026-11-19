-- This file should undo anything in `up.sql`
CREATE INDEX CONCURRENTLY IF NOT EXISTS payment_attempt_attempt_id_merchant_id_index ON payment_attempt (attempt_id, merchant_id);
