-- Increasing a varchar's length only changes the column's definition, existing rows and the
-- primary key index are not rewritten.
ALTER TABLE process_tracker ALTER COLUMN id TYPE VARCHAR(255);
