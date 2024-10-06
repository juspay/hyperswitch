-- The following queries must be run before the newer version of the application is deployed.
ALTER TABLE events
    ADD COLUMN merchant_id VARCHAR(64) DEFAULT NULL,
    ADD COLUMN business_profile_id VARCHAR(64) DEFAULT NULL,
    ADD COLUMN primary_object_created_at TIMESTAMP DEFAULT NULL,
    ADD COLUMN idempotent_event_id VARCHAR(64) DEFAULT NULL,
    ADD COLUMN initial_attempt_id VARCHAR(64) DEFAULT NULL,
    ADD COLUMN request BYTEA DEFAULT NULL,
    ADD COLUMN response BYTEA DEFAULT NULL;

UPDATE events
SET idempotent_event_id = event_id
WHERE idempotent_event_id IS NULL;

UPDATE events
SET initial_attempt_id = event_id
WHERE initial_attempt_id IS NULL;

ALTER TABLE events
ADD CONSTRAINT idempotent_event_id_unique UNIQUE (idempotent_event_id);

-- The following queries must be run after the newer version of the application is deployed.
-- Running these queries can even be deferred for some time (a couple of weeks or even a month) until the
-- new version being deployed is considered stable.
-- Make `event_id` primary key instead of `id`
ALTER TABLE events DROP CONSTRAINT events_pkey;

ALTER TABLE events
ADD PRIMARY KEY (event_id);

ALTER TABLE events DROP CONSTRAINT event_id_unique;

-- Dropping unused columns
ALTER TABLE events
    DROP COLUMN id,
    DROP COLUMN intent_reference_id;
