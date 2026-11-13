-- Fails while any event holds a type, class or object type that the database enums do not list,
-- such as the revenue recovery ones.
ALTER TABLE events
    ALTER COLUMN event_type TYPE "EventType" USING event_type::"EventType",
    ALTER COLUMN event_class TYPE "EventClass" USING event_class::"EventClass",
    ALTER COLUMN primary_object_type TYPE "EventObjectType" USING primary_object_type::"EventObjectType";
