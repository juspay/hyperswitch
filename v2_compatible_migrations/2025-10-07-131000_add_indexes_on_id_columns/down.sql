-- Drop unique indexes on id columns
-- This will remove the unique indexes created for id column performance optimization and data integrity

-- tracker tables
DROP INDEX IF EXISTS customers_id_index;

DROP INDEX IF EXISTS payment_intent_id_index;

DROP INDEX IF EXISTS payment_attempt_id_index;

DROP INDEX IF EXISTS payment_methods_id_index;

DROP INDEX IF EXISTS refund_id_index;

-- Accounts tables
DROP INDEX IF EXISTS business_profile_id_index;

DROP INDEX IF EXISTS merchant_account_id_index;

DROP INDEX IF EXISTS merchant_connector_account_id_index;

DROP INDEX IF EXISTS organization_id_index;
