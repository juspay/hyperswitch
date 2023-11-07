-- This file should undo anything in `up.sql`
ALTER TABLE events DROP CONSTRAINT event_id_unique;
