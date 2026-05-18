-- Add initiator_merchant_id column to events table: the merchant that
-- initiated the operation for which the webhook event was generated.
ALTER TABLE events
ADD COLUMN IF NOT EXISTS initiator_merchant_id VARCHAR(64);

-- Backfill existing rows by defaulting initiator_merchant_id to merchant_id.
-- This backfill should be executed again after deployment is complete.
UPDATE events
SET initiator_merchant_id = merchant_id
WHERE initiator_merchant_id IS NULL;

CREATE UNIQUE INDEX IF NOT EXISTS events_initiator_merchant_id_event_id_index
    ON events (initiator_merchant_id, event_id);

CREATE INDEX IF NOT EXISTS events_initiator_merchant_id_initial_attempt_id_index
    ON events (initiator_merchant_id, initial_attempt_id);
