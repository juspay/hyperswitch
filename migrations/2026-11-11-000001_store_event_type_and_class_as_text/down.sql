-- Fails while any event holds a type or class that the database enums do not list, such as the
-- revenue recovery ones. Postgres cannot drop an enum value, so `recovery_retry_details` stays.
ALTER TABLE events
    ALTER COLUMN event_type TYPE "EventType" USING event_type::"EventType",
    ALTER COLUMN event_class TYPE "EventClass" USING event_class::"EventClass";
