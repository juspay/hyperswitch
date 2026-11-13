-- Event types and classes are stored as text and typed in the application, so a new type or class
-- needs no migration.
ALTER TABLE events
    ALTER COLUMN event_type TYPE VARCHAR(64) USING event_type::TEXT,
    ALTER COLUMN event_class TYPE VARCHAR(64) USING event_class::TEXT;

ALTER TYPE "EventObjectType"
ADD VALUE IF NOT EXISTS 'recovery_retry_details';
