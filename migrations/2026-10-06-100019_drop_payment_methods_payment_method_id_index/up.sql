-- Redundant: on v1 payment_methods_pkey is already UNIQUE (payment_method_id).
-- On v2-compatible deployments that primary key is dropped by
-- v2_compatible_migrations/2024-08-28-081757, which creates
-- idx_payment_methods_payment_method_id in its place -- the target index name.
DROP INDEX CONCURRENTLY IF EXISTS payment_methods_payment_method_id_index;
