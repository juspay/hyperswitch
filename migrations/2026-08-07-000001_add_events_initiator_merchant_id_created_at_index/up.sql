-- Your SQL goes here
-- Supports the webhook events list/count queries, which filter by initiator
-- merchant, restrict `created_at` to a time range and sort by `created_at DESC`.
-- Partial on the initial-attempt predicate so retry rows are excluded from the
-- index entirely.
--
-- NOTE: `initiator_merchant_id` is NULL on the majority of existing rows. Until
-- that column is backfilled, the list query's `initiator_merchant_id IS NULL AND
-- merchant_id = ...` fallback branch is not served by this index.
CREATE INDEX CONCURRENTLY IF NOT EXISTS events_initiator_merchant_id_created_at_initial_index
ON events (initiator_merchant_id, created_at)
WHERE event_id = initial_attempt_id;
