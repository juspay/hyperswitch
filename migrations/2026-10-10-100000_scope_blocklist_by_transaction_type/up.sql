-- Existing rows are payment-scoped
ALTER TABLE blocklist ADD COLUMN transaction_type TEXT;

UPDATE blocklist SET transaction_type = 'payment' WHERE transaction_type IS NULL;
