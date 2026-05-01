------------------------ Organization -----------------------
-- Drop index on org_id in organization table
DROP INDEX IF EXISTS idx_organization_org_id;
-- Re-add primary key constraint on `organization`
ALTER TABLE organization 
    ADD CONSTRAINT organization_pkey PRIMARY KEY (org_id),
    ALTER COLUMN org_id SET NOT NULL;

------------------------ Merchant Account -------------------
DROP INDEX IF EXISTS idx_merchant_account_merchant_id;
ALTER TABLE merchant_account 
    ADD PRIMARY KEY (merchant_id),
    ALTER COLUMN primary_business_details SET NOT NULL,
    ALTER COLUMN is_recon_enabled SET NOT NULL,
    ALTER COLUMN is_recon_enabled SET DEFAULT FALSE;

------------------------ Business Profile -------------------
DROP INDEX IF EXISTS idx_business_profile_profile_id;
ALTER TABLE business_profile 
    ADD PRIMARY KEY (profile_id);
    

---------------- Merchant Connector Account -----------------
DROP INDEX IF EXISTS idx_merchant_connector_account_merchant_connector_id;
ALTER TABLE merchant_connector_account 
    ADD PRIMARY KEY (merchant_connector_id);

------------------------ Customers --------------------------
DROP INDEX IF EXISTS idx_customers_merchant_id_customer_id;
ALTER TABLE customers 
    ADD PRIMARY KEY (merchant_id, customer_id);

---------------------- Payment Intent -----------------------
DROP INDEX IF EXISTS idx_payment_intent_payment_id_merchant_id;
ALTER TABLE payment_intent 
    ADD PRIMARY KEY (payment_id, merchant_id),
    ALTER COLUMN active_attempt_id SET NOT NULL,
    ALTER COLUMN active_attempt_id SET DEFAULT 'xxx';

---------------------- Payment Attempt ----------------------
DROP INDEX IF EXISTS idx_payment_attempt_attempt_id_merchant_id;
ALTER TABLE payment_attempt 
    ADD PRIMARY KEY (attempt_id, merchant_id),
    ALTER COLUMN amount SET NOT NULL;

ALTER TABLE payment_attempt
    ALTER COLUMN confirm SET NOT NULL;

---------------------- Payment Methods ----------------------
DROP INDEX IF EXISTS idx_payment_methods_payment_method_id;
ALTER TABLE payment_methods 
    ADD PRIMARY KEY (payment_method_id);

---------------------- Refunds ----------------------
DROP INDEX IF EXISTS idx_refund_refund_id_merchant_id;
ALTER TABLE refund
    ADD PRIMARY KEY (refund_id,merchant_id);
ALTER TABLE refund
    ALTER COLUMN internal_reference_id SET NOT NULL;