-- Your SQL goes here
-- Exact duplicate of payment_intent_pkey, which has been (payment_id, merchant_id) since the
-- 2024-06-03-090859_make_id_as_optional_for_payments migration. Both are unique over the same
-- columns in the same order, so every lookup this index could serve is served by the primary key.
DROP INDEX CONCURRENTLY IF EXISTS payment_intent_payment_id_merchant_id_index;
