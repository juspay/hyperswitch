CREATE INDEX CONCURRENTLY IF NOT EXISTS events_initiator_merchant_id_initial_attempt_id_index
    ON events (initiator_merchant_id, initial_attempt_id);
