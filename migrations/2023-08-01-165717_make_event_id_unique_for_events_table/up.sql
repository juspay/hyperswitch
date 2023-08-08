-- Your SQL goes here
ALTER TABLE events
ADD CONSTRAINT event_id_unique UNIQUE (event_id);
