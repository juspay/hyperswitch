DROP INDEX IF EXISTS events_initiator_merchant_id_initial_attempt_id_index;
DROP INDEX IF EXISTS events_initiator_merchant_id_event_id_index;

ALTER TABLE events
DROP COLUMN IF EXISTS initiator_merchant_id;
