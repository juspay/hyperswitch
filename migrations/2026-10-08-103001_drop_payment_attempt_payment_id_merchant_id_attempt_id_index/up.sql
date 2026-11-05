-- Your SQL goes here
-- Redundant with payment_attempt_pkey (attempt_id, merchant_id): because that key is already
-- unique, uniqueness over the (payment_id, merchant_id, attempt_id) superset is implied, and a
-- lookup on all three columns resolves to a single row through the primary key. Scans on the
-- leading (payment_id, merchant_id) prefix remain served by
-- payment_attempt_payment_id_merchant_id_index.
DROP INDEX CONCURRENTLY IF EXISTS payment_attempt_payment_id_merchant_id_attempt_id_index;
