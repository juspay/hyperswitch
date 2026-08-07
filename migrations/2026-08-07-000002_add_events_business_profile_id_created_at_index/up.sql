-- Your SQL goes here
-- Supports the profile-scoped webhook events list/count queries, which filter by
-- business profile, restrict `created_at` to a time range and sort by
-- `created_at DESC`. Partial on the initial-attempt predicate so retry rows are
-- excluded from the index entirely.
CREATE INDEX CONCURRENTLY IF NOT EXISTS events_business_profile_id_created_at_initial_index
ON events (business_profile_id, created_at)
WHERE event_id = initial_attempt_id;
