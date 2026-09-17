-- Your SQL goes here
CREATE INDEX CONCURRENTLY IF NOT EXISTS events_initiator_merchant_id_created_at_initial_index
ON events (initiator_merchant_id, created_at)
WHERE event_id = initial_attempt_id;
