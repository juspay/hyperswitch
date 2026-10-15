-- Existing rows are payment-scoped
ALTER TABLE blocklist ADD COLUMN transaction_type TEXT;

-- For large tables, this backfill should be executed in batches.
-- The UPDATE below is not batched and holds the ALTER TABLE lock until commit.
UPDATE blocklist SET transaction_type = 'payment' WHERE transaction_type IS NULL;
