-- Drop not null constraint on org_id in organization table
-- Drop not null constraint on org_id in organization table
ALTER TABLE organization 
    DROP CONSTRAINT organization_pkey,
    ALTER COLUMN org_id DROP NOT NULL;

-- Drop not null in merchant_account table for v1 columns that are dropped in v2
ALTER TABLE merchant_account 
    DROP CONSTRAINT merchant_account_pkey,
    ALTER COLUMN merchant_id DROP NOT NULL,
    ALTER COLUMN primary_business_details DROP NOT NULL,
    ALTER COLUMN is_recon_enabled DROP NOT NULL;

ALTER TABLE business_profile 
    DROP CONSTRAINT business_profile_pkey,
    ALTER COLUMN profile_id DROP NOT NULL;

ALTER TABLE merchant_connector_account 
    DROP CONSTRAINT merchant_connector_account_pkey,
    ALTER COLUMN merchant_connector_id DROP NOT NULL;

ALTER TABLE customers 
    DROP CONSTRAINT customers_pkey,
    ALTER COLUMN customer_id DROP NOT NULL;

ALTER TABLE payment_intent 
    DROP CONSTRAINT payment_intent_pkey,
    ALTER COLUMN payment_id DROP NOT NULL,
    ALTER COLUMN active_attempt_id DROP NOT NULL,
    ALTER COLUMN active_attempt_id DROP DEFAULT;

ALTER TABLE payment_attempt 
    DROP CONSTRAINT payment_attempt_pkey,
    ALTER COLUMN attempt_id DROP NOT NULL,
    ALTER COLUMN amount DROP NOT NULL;

ALTER TABLE payment_methods 
    DROP CONSTRAINT payment_methods_pkey,
    ALTER COLUMN payment_method_id DROP NOT NULL;
