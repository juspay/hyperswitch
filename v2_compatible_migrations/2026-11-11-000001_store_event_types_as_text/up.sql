-- Event types, classes and object types are stored as text and typed in the application, so a new
-- one needs no migration.
ALTER TABLE events
    ALTER COLUMN event_type TYPE VARCHAR(64) USING event_type::TEXT,
    ALTER COLUMN event_class TYPE VARCHAR(64) USING event_class::TEXT,
    ALTER COLUMN primary_object_type TYPE VARCHAR(64) USING primary_object_type::TEXT;
