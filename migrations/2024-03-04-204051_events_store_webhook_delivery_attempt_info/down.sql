-- The following queries must be run before the older version of the application is deployed.
-- Remove `event_id` as primary key and add unique constraint
ALTER TABLE events DROP CONSTRAINT events_pkey;

ALTER TABLE events
ADD CONSTRAINT event_id_unique UNIQUE (event_id);

-- Adding back unused columns, and make `id` as primary key
ALTER TABLE events
ADD COLUMN id SERIAL PRIMARY KEY,
    ADD COLUMN intent_reference_id VARCHAR(64) DEFAULT NULL;

-- The following queries must be run after the older version of the application is deployed.
ALTER TABLE events DROP COLUMN idempotent_event_id,
    DROP COLUMN initial_attempt_id,
    DROP COLUMN request,
    DROP COLUMN response;
