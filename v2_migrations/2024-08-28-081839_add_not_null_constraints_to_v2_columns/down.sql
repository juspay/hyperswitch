-- This file should undo anything in `up.sql`
ALTER TABLE customers 
    ALTER COLUMN status DROP NOT NULL,
    ALTER COLUMN status DROP DEFAULT;

---------------------business_profile---------------------
ALTER TABLE business_profile ALTER COLUMN should_collect_cvv_during_payment DROP NOT NULL;


ALTER TABLE merchant_connector_account
    ALTER COLUMN profile_id DROP NOT NULL;

ALTER TABLE payment_intent
    ALTER COLUMN profile_id DROP NOT NULL,
    ALTER COLUMN currency DROP NOT NULL,
    ALTER COLUMN client_secret DROP NOT NULL,
    ALTER COLUMN session_expiry DROP NOT NULL;

ALTER TABLE payment_attempt
    ALTER COLUMN net_amount DROP NOT NULL;

-- This migration is to make fields mandatory in payment_attempt table
ALTER TABLE payment_attempt
    ALTER COLUMN net_amount DROP NOT NULL,
    ALTER COLUMN authentication_type DROP NOT NULL,
    ALTER COLUMN payment_method_type_v2 DROP NOT NULL,
    ALTER COLUMN payment_method_subtype DROP NOT NULL;