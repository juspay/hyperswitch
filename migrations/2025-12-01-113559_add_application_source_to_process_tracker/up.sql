-- Add the application_source column to the process_tracker table if it does not exist
ALTER TABLE process_tracker ADD COLUMN IF NOT EXISTS application_source VARCHAR(64);
