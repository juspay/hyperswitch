-- The column is also added by `v2_compatible_migrations` (`add_v2_columns`) for deployments that
-- serve the v2 customers API. `IF NOT EXISTS` keeps both migrations independent of the order they
-- are run in, and makes this one a no-op wherever the column is already present.
ALTER TABLE customers
ADD COLUMN IF NOT EXISTS merchant_reference_id VARCHAR(64);
