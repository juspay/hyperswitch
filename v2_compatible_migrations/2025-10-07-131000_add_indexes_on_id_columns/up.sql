-- Create unique indexes on id columns for better query performance and data integrity

-- Tracker Tables
CREATE UNIQUE INDEX IF NOT EXISTS customers_id_index ON customers (id);

CREATE UNIQUE INDEX IF NOT EXISTS payment_intent_id_index ON payment_intent (id);

CREATE UNIQUE INDEX IF NOT EXISTS payment_attempt_id_index ON payment_attempt (id);

CREATE UNIQUE INDEX IF NOT EXISTS payment_methods_id_index ON payment_methods (id);

CREATE UNIQUE INDEX IF NOT EXISTS refund_id_index ON refund (id);

-- Accounts Tables
CREATE UNIQUE INDEX IF NOT EXISTS business_profile_id_index ON business_profile (id);

CREATE UNIQUE INDEX IF NOT EXISTS merchant_account_id_index ON merchant_account (id);

CREATE UNIQUE INDEX IF NOT EXISTS merchant_connector_account_id_index ON merchant_connector_account (id);

CREATE UNIQUE INDEX IF NOT EXISTS organization_id_index ON organization (id);
