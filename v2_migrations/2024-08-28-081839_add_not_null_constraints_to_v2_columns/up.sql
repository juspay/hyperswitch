-- Your SQL goes here

ALTER TABLE customers 
    ALTER COLUMN status SET NOT NULL,
    ALTER COLUMN status SET DEFAULT 'active';

---------------------business_profile---------------------
ALTER TABLE business_profile ALTER COLUMN should_collect_cvv_during_payment SET NOT NULL;

-- This migration is to make profile_id mandatory in mca table
ALTER TABLE merchant_connector_account
    ALTER COLUMN profile_id SET NOT NULL;

-- This migration is to make fields mandatory in payment_intent table
ALTER TABLE payment_intent
    ALTER COLUMN profile_id SET NOT NULL,
    ALTER COLUMN currency SET NOT NULL,
    ALTER COLUMN client_secret SET NOT NULL,
    ALTER COLUMN session_expiry SET NOT NULL;

-- This migration is to make fields mandatory in payment_attempt table
ALTER TABLE payment_attempt
    ALTER COLUMN net_amount SET NOT NULL,
    ALTER COLUMN authentication_type SET NOT NULL,
    ALTER COLUMN payment_method_type_v2 SET NOT NULL,
    ALTER COLUMN payment_method_subtype SET NOT NULL;