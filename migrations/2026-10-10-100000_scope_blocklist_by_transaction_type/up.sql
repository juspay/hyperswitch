-- Commit the column addition before the following migration backfills rows.
ALTER TABLE blocklist ADD COLUMN transaction_type TEXT;
