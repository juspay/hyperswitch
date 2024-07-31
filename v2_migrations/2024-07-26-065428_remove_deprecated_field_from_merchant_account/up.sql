-- Your SQL goes here
ALTER TABLE merchant_account DROP COLUMN return_url;

ALTER TABLE merchant_account DROP enable_payment_response_hash;

ALTER TABLE merchant_account DROP payment_response_hash_key;

ALTER TABLE merchant_account DROP redirect_to_merchant_with_http_post;

ALTER TABLE merchant_account DROP sub_merchants_enabled;

ALTER TABLE merchant_account DROP parent_merchant_id;

ALTER TABLE merchant_account DROP primary_business_details;

-- ALTER TABLE merchant_account DROP locker_id;
