ALTER TABLE address
    ALTER COLUMN address_id TYPE VARCHAR(255),
    ALTER COLUMN city TYPE VARCHAR(255),
    ALTER COLUMN country TYPE VARCHAR(255),
    ALTER COLUMN state TYPE VARCHAR(255),
    ALTER COLUMN zip TYPE VARCHAR(255),
    ALTER COLUMN phone_number TYPE VARCHAR(255),
    ALTER COLUMN country_code TYPE VARCHAR(255),
    ALTER COLUMN customer_id TYPE VARCHAR(255),
    ALTER COLUMN merchant_id TYPE VARCHAR(255);

ALTER TABLE connector_response RENAME COLUMN attempt_id TO txn_id;

ALTER TABLE connector_response
    ALTER COLUMN payment_id TYPE VARCHAR(255),
    ALTER COLUMN merchant_id TYPE VARCHAR(255),
    ALTER COLUMN txn_id TYPE VARCHAR(255),
    ALTER COLUMN connector_name TYPE VARCHAR(32),
    ALTER COLUMN connector_transaction_id TYPE VARCHAR(255);

ALTER TABLE customers
    ALTER COLUMN customer_id TYPE VARCHAR(255),
    ALTER COLUMN merchant_id TYPE VARCHAR(255),
    ALTER COLUMN phone TYPE VARCHAR(255),
    ALTER COLUMN phone_country_code TYPE VARCHAR(255);

ALTER TABLE events
    ALTER COLUMN event_id TYPE VARCHAR(255),
    ALTER COLUMN intent_reference_id TYPE VARCHAR(255),
    ALTER COLUMN primary_object_id TYPE VARCHAR(255);

ALTER TABLE mandate RENAME COLUMN previous_attempt_id to previous_transaction_id;

ALTER TABLE mandate
    ALTER COLUMN mandate_id TYPE VARCHAR(255),
    ALTER COLUMN customer_id TYPE VARCHAR(255),
    ALTER COLUMN merchant_id TYPE VARCHAR(255),
    ALTER COLUMN payment_method_id TYPE VARCHAR(255),
    ALTER COLUMN customer_ip_address TYPE VARCHAR(255),
    ALTER COLUMN network_transaction_id TYPE VARCHAR(255),
    ALTER COLUMN previous_transaction_id TYPE VARCHAR(255),
    ALTER COLUMN connector TYPE VARCHAR(255),
    ALTER COLUMN connector_mandate_id TYPE VARCHAR(255);

ALTER TABLE merchant_account
    ALTER COLUMN merchant_id TYPE VARCHAR(255),
    ALTER COLUMN api_key TYPE VARCHAR(255),
    ALTER COLUMN merchant_name TYPE VARCHAR(255),
    ALTER COLUMN parent_merchant_id TYPE VARCHAR(255),
    ALTER COLUMN publishable_key TYPE VARCHAR(255);

ALTER TABLE merchant_connector_account
    ALTER COLUMN merchant_id TYPE VARCHAR(255),
    ALTER COLUMN connector_name TYPE VARCHAR(255);

ALTER TABLE payment_attempt
    ALTER COLUMN payment_id TYPE VARCHAR(255),
    ALTER COLUMN merchant_id TYPE VARCHAR(255),
    ALTER COLUMN attempt_id TYPE VARCHAR(255),
    ALTER COLUMN connector TYPE VARCHAR(255),
    ALTER COLUMN payment_method_id TYPE VARCHAR(255),
    ALTER COLUMN connector_transaction_id TYPE VARCHAR(255),
    ALTER COLUMN mandate_id TYPE VARCHAR(255),
    ALTER COLUMN payment_token TYPE VARCHAR(255);

ALTER TABLE payment_intent
    ALTER COLUMN payment_id TYPE VARCHAR(255),
    ALTER COLUMN merchant_id TYPE VARCHAR(255),
    ALTER COLUMN customer_id TYPE VARCHAR(255),
    ALTER COLUMN connector_id TYPE VARCHAR(255),
    ALTER COLUMN shipping_address_id TYPE VARCHAR(255),
    ALTER COLUMN billing_address_id TYPE VARCHAR(255),
    ALTER COLUMN client_secret TYPE VARCHAR(255);

ALTER TABLE payment_methods ADD COLUMN network_transaction_id VARCHAR(255);

ALTER TABLE payment_methods
    ALTER COLUMN customer_id TYPE VARCHAR(255),
    ALTER COLUMN merchant_id TYPE VARCHAR(255),
    ALTER COLUMN payment_method_id TYPE VARCHAR(255),
    ALTER COLUMN scheme TYPE VARCHAR(255),
    ALTER COLUMN token TYPE VARCHAR(255),
    ALTER COLUMN issuer_name TYPE VARCHAR(255),
    ALTER COLUMN issuer_country TYPE VARCHAR(255),
    ALTER COLUMN swift_code TYPE VARCHAR(255),
    ALTER COLUMN direct_debit_token TYPE VARCHAR(255),
    ALTER COLUMN network_transaction_id TYPE VARCHAR(255),
    ALTER COLUMN payment_method_issuer TYPE VARCHAR(255);

ALTER TABLE process_tracker
    ALTER COLUMN name TYPE VARCHAR(255),
    ALTER COLUMN runner TYPE VARCHAR(255);

ALTER TABLE refund RENAME COLUMN connector_transaction_id to transaction_id;
ALTER TABLE refund RENAME COLUMN connector_refund_id to pg_refund_id;

ALTER TABLE refund
    ALTER COLUMN internal_reference_id TYPE VARCHAR(255),
    ALTER COLUMN refund_id TYPE VARCHAR(255),
    ALTER COLUMN payment_id TYPE VARCHAR(255),
    ALTER COLUMN merchant_id TYPE VARCHAR(255),
    ALTER COLUMN attempt_id TYPE VARCHAR(255),
    ALTER COLUMN connector TYPE VARCHAR(255),
    ALTER COLUMN pg_refund_id TYPE VARCHAR(255),
    ALTER COLUMN external_reference_id TYPE VARCHAR(255),
    ALTER COLUMN refund_arn TYPE VARCHAR(255);

ALTER TABLE reverse_lookup
    ALTER COLUMN lookup_id TYPE VARCHAR(255),
    ALTER COLUMN sk_id TYPE VARCHAR(50),
    ALTER COLUMN pk_id TYPE VARCHAR(255),
    ALTER COLUMN source TYPE VARCHAR(30);
