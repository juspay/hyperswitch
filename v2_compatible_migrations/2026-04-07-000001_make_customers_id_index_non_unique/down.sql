-- Revert to unique index (rollback)
-- Note: This will fail if there are duplicate id values in the customers table.
DROP INDEX IF EXISTS customers_id_index;
CREATE UNIQUE INDEX IF NOT EXISTS customers_id_index ON customers (id);
