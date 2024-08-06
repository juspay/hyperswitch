-- Your SQL goes here
ALTER TABLE merchant_account DROP COLUMN return_url;

ALTER TABLE merchant_account DROP enable_payment_response_hash;

ALTER TABLE merchant_account DROP payment_response_hash_key;

ALTER TABLE merchant_account DROP redirect_to_merchant_with_http_post;

ALTER TABLE merchant_account DROP sub_merchants_enabled;

ALTER TABLE merchant_account DROP parent_merchant_id;

ALTER TABLE merchant_account DROP primary_business_details;

ALTER TABLE merchant_account DROP locker_id;

ALTER TABLE merchant_account DROP intent_fulfillment_time;

ALTER TABLE merchant_account DROP default_profile;

ALTER TABLE merchant_account DROP payment_link_config;

ALTER TABLE merchant_account DROP pm_collect_link_config;

ALTER TABLE merchant_account DROP is_recon_enabled;

ALTER TABLE merchant_account DROP webhook_details;

ALTER TABLE merchant_account DROP routing_algorithm;

ALTER TABLE merchant_account DROP frm_routing_algorithm;

ALTER TABLE merchant_account DROP payout_routing_algorithm;
