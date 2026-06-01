CREATE UNIQUE INDEX CONCURRENTLY IF NOT EXISTS events_initiator_merchant_id_event_id_index
    ON events (initiator_merchant_id, event_id);
