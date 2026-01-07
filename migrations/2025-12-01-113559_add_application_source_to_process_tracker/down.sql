-- Drop the application_source column from the process_tracker table if it exists
ALTER TABLE process_tracker DROP COLUMN IF EXISTS application_source;
