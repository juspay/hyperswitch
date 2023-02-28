-- Types
CREATE TYPE "AttemptStatus" AS ENUM (
    'started',
    'authentication_failed',
    'juspay_declined',
    'pending_vbv',
    'vbv_successful',
    'authorized',
    'authorization_failed',
    'charged',
    'authorizing',
    'cod_initiated',
    'voided',
    'void_initiated',
    'capture_initiated',
    'capture_failed',
    'void_failed',
    'auto_refunded',
    'partial_charged',
    'pending',
    'failure',
    'payment_method_awaited',
    'confirmation_awaited'
);

CREATE TYPE "AuthenticationType" AS ENUM ('three_ds', 'no_three_ds');

CREATE TYPE "CaptureMethod" AS ENUM ('automatic', 'manual', 'scheduled');

CREATE TYPE "ConnectorType" AS ENUM (
    'payment_processor',
    'payment_vas',
    'fin_operations',
    'fiz_operations',
    'networks',
    'banking_entities',
    'non_banking_finance'
);

CREATE TYPE "Currency" AS ENUM (
    'AED',
    'ALL',
    'AMD',
    'ARS',
    'AUD',
    'AWG',
    'AZN',
    'BBD',
    'BDT',
    'BHD',
    'BMD',
    'BND',
    'BOB',
    'BRL',
    'BSD',
    'BWP',
    'BZD',
    'CAD',
    'CHF',
    'CNY',
    'COP',
    'CRC',
    'CUP',
    'CZK',
    'DKK',
    'DOP',
    'DZD',
    'EGP',
    'ETB',
    'EUR',
    'FJD',
    'GBP',
    'GHS',
    'GIP',
    'GMD',
    'GTQ',
    'GYD',
    'HKD',
    'HNL',
    'HRK',
    'HTG',
    'HUF',
    'IDR',
    'ILS',
    'INR',
    'JMD',
    'JOD',
    'JPY',
    'KES',
    'KGS',
    'KHR',
    'KRW',
    'KWD',
    'KYD',
    'KZT',
    'LAK',
    'LBP',
    'LKR',
    'LRD',
    'LSL',
    'MAD',
    'MDL',
    'MKD',
    'MMK',
    'MNT',
    'MOP',
    'MUR',
    'MVR',
    'MWK',
    'MXN',
    'MYR',
    'NAD',
    'NGN',
    'NIO',
    'NOK',
    'NPR',
    'NZD',
    'OMR',
    'PEN',
    'PGK',
    'PHP',
    'PKR',
    'PLN',
    'QAR',
    'RUB',
    'SAR',
    'SCR',
    'SEK',
    'SGD',
    'SLL',
    'SOS',
    'SSP',
    'SVC',
    'SZL',
    'THB',
    'TTD',
    'TWD',
    'TZS',
    'USD',
    'UYU',
    'UZS',
    'YER',
    'ZAR'
);

CREATE TYPE "EventClass" AS ENUM ('payments');

CREATE TYPE "EventObjectType" AS ENUM ('payment_details');

CREATE TYPE "EventType" AS ENUM ('payment_succeeded');

CREATE TYPE "FutureUsage" AS ENUM ('on_session', 'off_session');

CREATE TYPE "IntentStatus" AS ENUM (
    'succeeded',
    'failed',
    'processing',
    'requires_customer_action',
    'requires_payment_method',
    'requires_confirmation'
);

CREATE TYPE "MandateStatus" AS ENUM (
    'active',
    'inactive',
    'pending',
    'revoked'
);

CREATE TYPE "MandateType" AS ENUM ('single_use', 'multi_use');

CREATE TYPE "PaymentFlow" AS ENUM (
    'vsc',
    'emi',
    'otp',
    'upi_intent',
    'upi_collect',
    'upi_scan_and_pay',
    'sdk'
);

CREATE TYPE "PaymentMethodIssuerCode" AS ENUM (
    'jp_hdfc',
    'jp_icici',
    'jp_googlepay',
    'jp_applepay',
    'jp_phonepe',
    'jp_wechat',
    'jp_sofort',
    'jp_giropay',
    'jp_sepa',
    'jp_bacs'
);

CREATE TYPE "PaymentMethodSubType" AS ENUM (
    'credit',
    'debit',
    'upi_intent',
    'upi_collect',
    'credit_card_installments',
    'pay_later_installments'
);

CREATE TYPE "PaymentMethodType" AS ENUM (
    'card',
    'bank_transfer',
    'netbanking',
    'upi',
    'open_banking',
    'consumer_finance',
    'wallet',
    'payment_container',
    'bank_debit',
    'pay_later'
);

CREATE TYPE "ProcessTrackerStatus" AS ENUM (
    'processing',
    'new',
    'pending',
    'process_started',
    'finish'
);

CREATE TYPE "RefundStatus" AS ENUM (
    'failure',
    'manual_review',
    'pending',
    'success',
    'transaction_failure'
);

CREATE TYPE "RefundType" AS ENUM (
    'instant_refund',
    'regular_refund',
    'retry_refund'
);

CREATE TYPE "RoutingAlgorithm" AS ENUM (
    'round_robin',
    'max_conversion',
    'min_cost',
    'custom'
);

-- Tables
CREATE TABLE address (
    id SERIAL,
    address_id VARCHAR(255) PRIMARY KEY,
    city VARCHAR(255),
    country VARCHAR(255),
    line1 VARCHAR(255),
    line2 VARCHAR(255),
    line3 VARCHAR(255),
    state VARCHAR(255),
    zip VARCHAR(255),
    first_name VARCHAR(255),
    last_name VARCHAR(255),
    phone_number VARCHAR(255),
    country_code VARCHAR(255),
    created_at TIMESTAMP NOT NULL DEFAULT now()::TIMESTAMP,
    modified_at TIMESTAMP NOT NULL DEFAULT now()::TIMESTAMP
);

CREATE TABLE configs (
    id SERIAL,
    key VARCHAR(255) NOT NULL,
    config TEXT NOT NULL,
    PRIMARY KEY (key)
);

CREATE TABLE customers (
    id SERIAL,
    customer_id VARCHAR(255) NOT NULL,
    merchant_id VARCHAR(255) NOT NULL,
    NAME VARCHAR(255),
    email VARCHAR(255),
    phone VARCHAR(255),
    phone_country_code VARCHAR(255),
    description VARCHAR(255),
    address JSON,
    created_at TIMESTAMP NOT NULL DEFAULT now()::TIMESTAMP,
    metadata JSON,
    PRIMARY KEY (customer_id, merchant_id)
);

CREATE TABLE events (
    id SERIAL PRIMARY KEY,
    event_id VARCHAR(255) NOT NULL,
    event_type "EventType" NOT NULL,
    event_class "EventClass" NOT NULL,
    is_webhook_notified BOOLEAN NOT NULL DEFAULT FALSE,
    intent_reference_id VARCHAR(255),
    primary_object_id VARCHAR(255) NOT NULL,
    primary_object_type "EventObjectType" NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT now()::TIMESTAMP
);

CREATE TABLE locker_mock_up (
    id SERIAL PRIMARY KEY,
    card_id VARCHAR(255) NOT NULL,
    external_id VARCHAR(255) NOT NULL,
    card_fingerprint VARCHAR(255) NOT NULL,
    card_global_fingerprint VARCHAR(255) NOT NULL,
    merchant_id VARCHAR(255) NOT NULL,
    card_number VARCHAR(255) NOT NULL,
    card_exp_year VARCHAR(255) NOT NULL,
    card_exp_month VARCHAR(255) NOT NULL,
    name_on_card VARCHAR(255),
    nickname VARCHAR(255),
    customer_id VARCHAR(255),
    duplicate BOOLEAN
);

CREATE TABLE mandate (
    id SERIAL PRIMARY KEY,
    mandate_id VARCHAR(255) NOT NULL,
    customer_id VARCHAR(255) NOT NULL,
    merchant_id VARCHAR(255) NOT NULL,
    payment_method_id VARCHAR(255) NOT NULL,
    mandate_status "MandateStatus" NOT NULL,
    mandate_type "MandateType" NOT NULL,
    customer_accepted_at TIMESTAMP,
    customer_ip_address VARCHAR(255),
    customer_user_agent VARCHAR(255),
    network_transaction_id VARCHAR(255),
    previous_transaction_id VARCHAR(255),
    created_at TIMESTAMP NOT NULL DEFAULT now()::TIMESTAMP
);

CREATE TABLE merchant_account (
    id SERIAL PRIMARY KEY,
    merchant_id VARCHAR(255) NOT NULL,
    api_key VARCHAR(255),
    return_url VARCHAR(255),
    enable_payment_response_hash BOOLEAN NOT NULL DEFAULT FALSE,
    payment_response_hash_key VARCHAR(255) DEFAULT NULL,
    redirect_to_merchant_with_http_post BOOLEAN NOT NULL DEFAULT FALSE,
    merchant_name VARCHAR(255),
    merchant_details JSON,
    webhook_details JSON,
    routing_algorithm "RoutingAlgorithm",
    custom_routing_rules JSON,
    sub_merchants_enabled BOOLEAN DEFAULT FALSE,
    parent_merchant_id VARCHAR(255),
    publishable_key VARCHAR(255)
);

CREATE TABLE merchant_connector_account (
    id SERIAL PRIMARY KEY,
    merchant_id VARCHAR(255) NOT NULL,
    connector_name VARCHAR(255) NOT NULL,
    connector_account_details JSON NOT NULL,
    test_mode BOOLEAN,
    disabled BOOLEAN,
    merchant_connector_id SERIAL NOT NULL,
    payment_methods_enabled JSON [ ],
    connector_type "ConnectorType" NOT NULL DEFAULT 'payment_processor'::"ConnectorType"
);

CREATE TABLE payment_attempt (
    id SERIAL PRIMARY KEY,
    payment_id VARCHAR(255) NOT NULL,
    merchant_id VARCHAR(255) NOT NULL,
    txn_id VARCHAR(255) NOT NULL,
    status "AttemptStatus" NOT NULL,
    amount INTEGER NOT NULL,
    currency "Currency",
    save_to_locker BOOLEAN,
    connector VARCHAR(255) NOT NULL,
    error_message TEXT,
    offer_amount INTEGER,
    surcharge_amount INTEGER,
    tax_amount INTEGER,
    payment_method_id VARCHAR(255),
    payment_method "PaymentMethodType",
    payment_flow "PaymentFlow",
    redirect BOOLEAN,
    connector_transaction_id VARCHAR(255),
    capture_method "CaptureMethod",
    capture_on TIMESTAMP,
    confirm BOOLEAN NOT NULL,
    authentication_type "AuthenticationType",
    created_at TIMESTAMP NOT NULL,
    modified_at TIMESTAMP NOT NULL,
    last_synced TIMESTAMP
);

CREATE TABLE payment_intent (
    id SERIAL PRIMARY KEY,
    payment_id VARCHAR(255) NOT NULL,
    merchant_id VARCHAR(255) NOT NULL,
    status "IntentStatus" NOT NULL,
    amount INTEGER NOT NULL,
    currency "Currency",
    amount_captured INTEGER,
    customer_id VARCHAR(255),
    description VARCHAR(255),
    return_url VARCHAR(255),
    metadata JSONB DEFAULT '{}'::JSONB,
    connector_id VARCHAR(255),
    shipping_address_id VARCHAR(255),
    billing_address_id VARCHAR(255),
    statement_descriptor_name VARCHAR(255),
    statement_descriptor_suffix VARCHAR(255),
    created_at TIMESTAMP NOT NULL,
    modified_at TIMESTAMP NOT NULL,
    last_synced TIMESTAMP,
    setup_future_usage "FutureUsage",
    off_session BOOLEAN,
    client_secret VARCHAR(255)
);

CREATE TABLE payment_methods (
    id SERIAL PRIMARY KEY,
    customer_id VARCHAR(255) NOT NULL,
    merchant_id VARCHAR(255) NOT NULL,
    payment_method_id VARCHAR(255) NOT NULL,
    accepted_currency "Currency" [ ],
    scheme VARCHAR(255),
    token VARCHAR(255),
    cardholder_name VARCHAR(255),
    issuer_name VARCHAR(255),
    issuer_country VARCHAR(255),
    payer_country TEXT [ ],
    is_stored BOOLEAN,
    swift_code VARCHAR(255),
    direct_debit_token VARCHAR(255),
    network_transaction_id VARCHAR(255),
    created_at TIMESTAMP NOT NULL,
    last_modified TIMESTAMP NOT NULL,
    payment_method "PaymentMethodType" NOT NULL,
    payment_method_type "PaymentMethodSubType",
    payment_method_issuer VARCHAR(255),
    payment_method_issuer_code "PaymentMethodIssuerCode"
);

CREATE TABLE process_tracker (
    id VARCHAR(127) PRIMARY KEY,
    NAME VARCHAR(255),
    tag TEXT [ ] NOT NULL DEFAULT '{}'::TEXT [ ],
    runner VARCHAR(255),
    retry_count INTEGER NOT NULL,
    schedule_time TIMESTAMP,
    rule VARCHAR(255) NOT NULL,
    tracking_data JSON NOT NULL,
    business_status VARCHAR(255) NOT NULL,
    status "ProcessTrackerStatus" NOT NULL,
    event TEXT [ ] NOT NULL DEFAULT '{}'::TEXT [ ],
    created_at TIMESTAMP NOT NULL DEFAULT now(),
    updated_at TIMESTAMP NOT NULL DEFAULT now()
);

CREATE TABLE refund (
    id SERIAL PRIMARY KEY,
    internal_reference_id VARCHAR(255) NOT NULL,
    refund_id VARCHAR(255) NOT NULL,
    payment_id VARCHAR(255) NOT NULL,
    merchant_id VARCHAR(255) NOT NULL,
    transaction_id VARCHAR(255) NOT NULL,
    connector VARCHAR(255) NOT NULL,
    pg_refund_id VARCHAR(255),
    external_reference_id VARCHAR(255),
    refund_type "RefundType" NOT NULL,
    total_amount INTEGER NOT NULL,
    currency "Currency" NOT NULL,
    refund_amount INTEGER NOT NULL,
    refund_status "RefundStatus" NOT NULL,
    sent_to_gateway BOOLEAN NOT NULL DEFAULT FALSE,
    refund_error_message TEXT,
    metadata JSON,
    refund_arn VARCHAR(255),
    created_at TIMESTAMP NOT NULL,
    modified_at TIMESTAMP NOT NULL,
    description VARCHAR(255)
);

CREATE TABLE temp_card (
    id SERIAL PRIMARY KEY,
    date_created TIMESTAMP NOT NULL,
    txn_id VARCHAR(255),
    card_info JSON
);

-- Indices
CREATE INDEX customers_created_at_index ON customers (created_at);

CREATE UNIQUE INDEX merchant_account_api_key_index ON merchant_account (api_key);

CREATE UNIQUE INDEX merchant_account_merchant_id_index ON merchant_account (merchant_id);

CREATE UNIQUE INDEX merchant_account_publishable_key_index ON merchant_account (publishable_key);

CREATE INDEX merchant_connector_account_connector_type_index ON merchant_connector_account (connector_type);

CREATE INDEX merchant_connector_account_merchant_id_index ON merchant_connector_account (merchant_id);

CREATE UNIQUE INDEX payment_attempt_payment_id_merchant_id_index ON payment_attempt (payment_id, merchant_id);

CREATE UNIQUE INDEX payment_intent_payment_id_merchant_id_index ON payment_intent (payment_id, merchant_id);

CREATE INDEX payment_methods_created_at_index ON payment_methods (created_at);

CREATE INDEX payment_methods_customer_id_index ON payment_methods (customer_id);

CREATE INDEX payment_methods_last_modified_index ON payment_methods (last_modified);

CREATE INDEX payment_methods_payment_method_id_index ON payment_methods (payment_method_id);

CREATE INDEX refund_internal_reference_id_index ON refund (internal_reference_id);

CREATE INDEX refund_payment_id_merchant_id_index ON refund (payment_id, merchant_id);

CREATE INDEX refund_refund_id_index ON refund (refund_id);

CREATE UNIQUE INDEX refund_refund_id_merchant_id_index ON refund (refund_id, merchant_id);

CREATE INDEX temp_card_txn_id_index ON temp_card (txn_id);
