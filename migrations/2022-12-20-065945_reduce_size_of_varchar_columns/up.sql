ALTER TABLE address
    ALTER COLUMN address_id TYPE VARCHAR(64),
    ALTER COLUMN city TYPE VARCHAR(128),
    ALTER COLUMN country TYPE VARCHAR(64),
    ALTER COLUMN state TYPE VARCHAR(128),
    ALTER COLUMN zip TYPE VARCHAR(16),
    ALTER COLUMN phone_number TYPE VARCHAR(32),
    ALTER COLUMN country_code TYPE VARCHAR(8),
    ALTER COLUMN customer_id TYPE VARCHAR(64),
    ALTER COLUMN merchant_id TYPE VARCHAR(64);

ALTER TABLE connector_response RENAME COLUMN txn_id TO attempt_id;

ALTER TABLE connector_response
    ALTER COLUMN payment_id TYPE VARCHAR(64),
    ALTER COLUMN merchant_id TYPE VARCHAR(64),
    ALTER COLUMN attempt_id TYPE VARCHAR(64),
    ALTER COLUMN connector_name TYPE VARCHAR(64),
    ALTER COLUMN connector_transaction_id TYPE VARCHAR(128);

ALTER TABLE customers
    ALTER COLUMN customer_id TYPE VARCHAR(64),
    ALTER COLUMN merchant_id TYPE VARCHAR(64),
    ALTER COLUMN phone TYPE VARCHAR(32),
    ALTER COLUMN phone_country_code TYPE VARCHAR(8);

ALTER TABLE events
    ALTER COLUMN event_id TYPE VARCHAR(64),
    ALTER COLUMN intent_reference_id TYPE VARCHAR(64),
    ALTER COLUMN primary_object_id TYPE VARCHAR(64);

ALTER TABLE mandate RENAME COLUMN previous_transaction_id to previous_attempt_id;

ALTER TABLE mandate
    ALTER COLUMN mandate_id TYPE VARCHAR(64),
    ALTER COLUMN customer_id TYPE VARCHAR(64),
    ALTER COLUMN merchant_id TYPE VARCHAR(64),
    ALTER COLUMN payment_method_id TYPE VARCHAR(64),
    ALTER COLUMN customer_ip_address TYPE VARCHAR(64),
    ALTER COLUMN network_transaction_id TYPE VARCHAR(128),
    ALTER COLUMN previous_attempt_id TYPE VARCHAR(64),
    ALTER COLUMN connector TYPE VARCHAR(64),
    ALTER COLUMN connector_mandate_id TYPE VARCHAR(128);

ALTER TABLE merchant_account
    ALTER COLUMN merchant_id TYPE VARCHAR(64),
    ALTER COLUMN api_key TYPE VARCHAR(128),
    ALTER COLUMN merchant_name TYPE VARCHAR(128),
    ALTER COLUMN parent_merchant_id TYPE VARCHAR(64),
    ALTER COLUMN publishable_key TYPE VARCHAR(128);

ALTER TABLE merchant_connector_account
    ALTER COLUMN merchant_id TYPE VARCHAR(64),
    ALTER COLUMN connector_name TYPE VARCHAR(64);

ALTER TABLE payment_attempt
    ALTER COLUMN payment_id TYPE VARCHAR(64),
    ALTER COLUMN merchant_id TYPE VARCHAR(64),
    ALTER COLUMN attempt_id TYPE VARCHAR(64),
    ALTER COLUMN connector TYPE VARCHAR(64),
    ALTER COLUMN payment_method_id TYPE VARCHAR(64),
    ALTER COLUMN connector_transaction_id TYPE VARCHAR(128),
    ALTER COLUMN mandate_id TYPE VARCHAR(64),
    ALTER COLUMN payment_token TYPE VARCHAR(128);

ALTER TABLE payment_intent
    ALTER COLUMN payment_id TYPE VARCHAR(64),
    ALTER COLUMN merchant_id TYPE VARCHAR(64),
    ALTER COLUMN customer_id TYPE VARCHAR(64),
    ALTER COLUMN connector_id TYPE VARCHAR(64),
    ALTER COLUMN shipping_address_id TYPE VARCHAR(64),
    ALTER COLUMN billing_address_id TYPE VARCHAR(64),
    ALTER COLUMN client_secret TYPE VARCHAR(128);

ALTER TABLE payment_methods DROP COLUMN network_transaction_id;

ALTER TABLE payment_methods
    ALTER COLUMN customer_id TYPE VARCHAR(64),
    ALTER COLUMN merchant_id TYPE VARCHAR(64),
    ALTER COLUMN payment_method_id TYPE VARCHAR(64),
    ALTER COLUMN scheme TYPE VARCHAR(32),
    ALTER COLUMN token TYPE VARCHAR(128),
    ALTER COLUMN issuer_name TYPE VARCHAR(64),
    ALTER COLUMN issuer_country TYPE VARCHAR(64),
    ALTER COLUMN swift_code TYPE VARCHAR(32),
    ALTER COLUMN direct_debit_token TYPE VARCHAR(128),
    ALTER COLUMN payment_method_issuer TYPE VARCHAR(128);

ALTER TABLE process_tracker
    ALTER COLUMN name TYPE VARCHAR(64),
    ALTER COLUMN runner TYPE VARCHAR(64);

ALTER TABLE refund RENAME COLUMN transaction_id to connector_transaction_id;
ALTER TABLE refund RENAME COLUMN pg_refund_id to connector_refund_id;

ALTER TABLE refund
    ALTER COLUMN internal_reference_id TYPE VARCHAR(64),
    ALTER COLUMN refund_id TYPE VARCHAR(64),
    ALTER COLUMN payment_id TYPE VARCHAR(64),
    ALTER COLUMN merchant_id TYPE VARCHAR(64),
    ALTER COLUMN connector_transaction_id TYPE VARCHAR(128),
    ALTER COLUMN connector TYPE VARCHAR(64),
    ALTER COLUMN connector_refund_id TYPE VARCHAR(128),
    ALTER COLUMN external_reference_id TYPE VARCHAR(64),
    ALTER COLUMN refund_arn TYPE VARCHAR(128);

ALTER TABLE reverse_lookup
    ALTER COLUMN lookup_id TYPE VARCHAR(128),
    ALTER COLUMN sk_id TYPE VARCHAR(128),
    ALTER COLUMN pk_id TYPE VARCHAR(128),
    ALTER COLUMN source TYPE VARCHAR(128);
