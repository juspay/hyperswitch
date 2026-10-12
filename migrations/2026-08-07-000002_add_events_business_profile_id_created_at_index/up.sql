-- Your SQL goes here
CREATE INDEX CONCURRENTLY IF NOT EXISTS events_business_profile_id_created_at_initial_index
ON events (business_profile_id, created_at)
WHERE event_id = initial_attempt_id;
