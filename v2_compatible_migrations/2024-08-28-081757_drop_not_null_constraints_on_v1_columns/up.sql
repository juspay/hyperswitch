-- Drop not null constraints on not null columns of v1 which are either dropped or made nullable in v2.
------------------------ Organization -----------------------
ALTER TABLE organization 
    DROP CONSTRAINT organization_pkey,
    ALTER COLUMN org_id DROP NOT NULL;
-- Create index on org_id in organization table
-- This is done to nullify the effects of dropping primary key for v1
CREATE INDEX idx_organization_org_id ON organization (org_id);


------------------------ Merchant Account -------------------
-- Drop not null in merchant_account table for v1 columns that are dropped in v2
ALTER TABLE merchant_account 
    DROP CONSTRAINT merchant_account_pkey,
    ALTER COLUMN merchant_id DROP NOT NULL,
    ALTER COLUMN primary_business_details DROP NOT NULL,
    ALTER COLUMN is_recon_enabled DROP NOT NULL;

-- This is done to nullify the effects of dropping primary key for v1
CREATE INDEX idx_merchant_account_merchant_id ON merchant_account (merchant_id);

------------------------ Business Profile -------------------
ALTER TABLE business_profile 
    DROP CONSTRAINT business_profile_pkey,
    ALTER COLUMN profile_id DROP NOT NULL;

-- This is done to nullify the effects of dropping primary key for v1
CREATE INDEX idx_business_profile_profile_id ON business_profile (profile_id);

---------------- Merchant Connector Account -----------------
ALTER TABLE merchant_connector_account 
    DROP CONSTRAINT merchant_connector_account_pkey,
    ALTER COLUMN merchant_connector_id DROP NOT NULL;
-- This is done to nullify the effects of dropping primary key for v1
CREATE INDEX idx_merchant_connector_account_merchant_connector_id ON merchant_connector_account (merchant_connector_id);

------------------------ Customers --------------------------
ALTER TABLE customers 
    DROP CONSTRAINT customers_pkey,
    ALTER COLUMN customer_id DROP NOT NULL;
-- This is done to nullify the effects of dropping primary key for v1
CREATE INDEX idx_customers_merchant_id_customer_id ON customers (merchant_id, customer_id);

---------------------- Payment Intent -----------------------
ALTER TABLE payment_intent 
    DROP CONSTRAINT payment_intent_pkey,
    ALTER COLUMN payment_id DROP NOT NULL,
    ALTER COLUMN active_attempt_id DROP NOT NULL,
    ALTER COLUMN active_attempt_id DROP DEFAULT;
-- This is done to nullify the effects of dropping primary key for v1
CREATE INDEX idx_payment_intent_payment_id_merchant_id ON payment_intent (payment_id, merchant_id);

---------------------- Payment Attempt ----------------------
ALTER TABLE payment_attempt 
    DROP CONSTRAINT payment_attempt_pkey,
    ALTER COLUMN attempt_id DROP NOT NULL,
    ALTER COLUMN amount DROP NOT NULL;
-- This is done to nullify the effects of dropping primary key for v1
CREATE INDEX idx_payment_attempt_attempt_id_merchant_id ON payment_attempt (attempt_id, merchant_id);

ALTER TABLE payment_attempt
    ALTER COLUMN confirm DROP NOT NULL;

---------------------- Payment Methods ----------------------
ALTER TABLE payment_methods 
    DROP CONSTRAINT payment_methods_pkey,
    ALTER COLUMN payment_method_id DROP NOT NULL;
-- This is done to nullify the effects of dropping primary key for v1
CREATE INDEX idx_payment_methods_payment_method_id ON payment_methods (payment_method_id);

---------------------- Refunds ----------------------
ALTER TABLE refund
    DROP CONSTRAINT refund_pkey,
    ALTER COLUMN refund_id DROP NOT NULL;

ALTER TABLE refund
    ALTER COLUMN internal_reference_id DROP NOT NULL;
-- This is done to nullify the effects of dropping primary key for v1
CREATE INDEX idx_refund_refund_id_merchant_id ON refund (refund_id, merchant_id);