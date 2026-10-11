-- Your SQL goes here
-- Exact duplicate of payment_attempt_pkey, which has been (attempt_id, merchant_id) since
-- the 2024-06-03-090859_make_id_as_optional_for_payments migration. This index was added as a
-- perf index in 2023-08-23 when the primary key was still (id), and became redundant after that.
DROP INDEX CONCURRENTLY IF EXISTS payment_attempt_attempt_id_merchant_id_index;
