-- Intentionally a no-op. `customers.merchant_reference_id` is shared with the v2 customers API
-- (see `v2_compatible_migrations/2024-08-28-081721_add_v2_columns`), which stores the merchant
-- supplied customer id in it. Dropping the column here would destroy that data.
SELECT 1;
