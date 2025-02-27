------------------------ Organization -----------------------
-- Re-add primary key constraint on `organization`
ALTER TABLE organization 
    ADD CONSTRAINT organization_pkey PRIMARY KEY (org_id),
    ALTER COLUMN org_id SET NOT NULL;

------------------------ Merchant Account -------------------
ALTER TABLE merchant_account 
    ADD PRIMARY KEY (merchant_id),
    ALTER COLUMN primary_business_details SET NOT NULL,
    ALTER COLUMN is_recon_enabled SET NOT NULL,
    ALTER COLUMN is_recon_enabled SET DEFAULT FALSE;

------------------------ Business Profile -------------------
ALTER TABLE business_profile 
    ADD PRIMARY KEY (profile_id);

---------------- Merchant Connector Account -----------------
ALTER TABLE merchant_connector_account 
    ADD PRIMARY KEY (merchant_connector_id);

------------------------ Customers --------------------------
ALTER TABLE customers 
    ADD PRIMARY KEY (merchant_id, customer_id);

---------------------- Payment Intent -----------------------
ALTER TABLE payment_intent 
    ADD PRIMARY KEY (payment_id, merchant_id),
    ALTER COLUMN active_attempt_id SET NOT NULL,
    ALTER COLUMN active_attempt_id SET DEFAULT 'xxx';

---------------------- Payment Attempt ----------------------
ALTER TABLE payment_attempt 
    ADD PRIMARY KEY (attempt_id, merchant_id),
    ALTER COLUMN amount SET NOT NULL;

---------------------- Payment Methods ----------------------
ALTER TABLE payment_methods 
    ADD PRIMARY KEY (payment_method_id);
