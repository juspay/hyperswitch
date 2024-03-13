CREATE TABLE payment_attempts_queue (
    `payment_id` String,
    `merchant_id` String,
    `attempt_id` String,
    `status` LowCardinality(String),
    `amount` Nullable(UInt32),
    `currency` LowCardinality(Nullable(String)),
    `connector` LowCardinality(Nullable(String)),
    `save_to_locker` Nullable(Bool),
    `error_message` Nullable(String),
    `offer_amount` Nullable(UInt32),
    `surcharge_amount` Nullable(UInt32),
    `tax_amount` Nullable(UInt32),
    `payment_method_id` Nullable(String),
    `payment_method` LowCardinality(Nullable(String)),
    `payment_method_type` LowCardinality(Nullable(String)),
    `connector_transaction_id` Nullable(String),
    `capture_method` LowCardinality(Nullable(String)),
    `capture_on` Nullable(DateTime) CODEC(T64, LZ4),
    `confirm` Bool,
    `authentication_type` LowCardinality(Nullable(String)),
    `cancellation_reason` Nullable(String),
    `amount_to_capture` Nullable(UInt32),
    `mandate_id` Nullable(String),
    `browser_info` Nullable(String),
    `error_code` Nullable(String),
    `connector_metadata` Nullable(String),
    `payment_experience` Nullable(String),
    `created_at` DateTime CODEC(T64, LZ4),
    `last_synced` Nullable(DateTime) CODEC(T64, LZ4),
    `modified_at` DateTime CODEC(T64, LZ4),
    `payment_method_data` Nullable(String),
    `error_reason` Nullable(String),
    `multiple_capture_count` Nullable(Int16),
    `amount_capturable` Nullable(UInt64) ,
    `merchant_connector_id`  Nullable(String),
    `net_amount` Nullable(UInt64) ,
    `unified_code`  Nullable(String),
    `unified_message`  Nullable(String),
    `mandate_data`  Nullable(String),
    `sign_flag` Int8
) ENGINE = Kafka SETTINGS kafka_broker_list = 'kafka0:29092',
kafka_topic_list = 'hyperswitch-payment-attempt-events',
kafka_group_name = 'hyper-c1',
kafka_format = 'JSONEachRow',
kafka_handle_error_mode = 'stream';

CREATE TABLE payment_attempt_dist (
    `payment_id` String,
    `merchant_id` String,
    `attempt_id` String,
    `status` LowCardinality(String),
    `amount` Nullable(UInt32),
    `currency` LowCardinality(Nullable(String)),
    `connector` LowCardinality(Nullable(String)),
    `save_to_locker` Nullable(Bool),
    `error_message` Nullable(String),
    `offer_amount` Nullable(UInt32),
    `surcharge_amount` Nullable(UInt32),
    `tax_amount` Nullable(UInt32),
    `payment_method_id` Nullable(String),
    `payment_method` LowCardinality(Nullable(String)),
    `payment_method_type` LowCardinality(Nullable(String)),
    `connector_transaction_id` Nullable(String),
    `capture_method` Nullable(String),
    `capture_on` Nullable(DateTime) CODEC(T64, LZ4),
    `confirm` Bool,
    `authentication_type` LowCardinality(Nullable(String)),
    `cancellation_reason` Nullable(String),
    `amount_to_capture` Nullable(UInt32),
    `mandate_id` Nullable(String),
    `browser_info` Nullable(String),
    `error_code` Nullable(String),
    `connector_metadata` Nullable(String),
    `payment_experience` Nullable(String),
    `created_at` DateTime DEFAULT now() CODEC(T64, LZ4),
    `last_synced` Nullable(DateTime) CODEC(T64, LZ4),
    `modified_at` DateTime DEFAULT now() CODEC(T64, LZ4),
    `payment_method_data` Nullable(String),
    `error_reason` Nullable(String),
    `multiple_capture_count` Nullable(Int16),
    `amount_capturable` Nullable(UInt64) ,
    `merchant_connector_id`  Nullable(String),
    `net_amount` Nullable(UInt64) ,
    `unified_code`  Nullable(String),
    `unified_message`  Nullable(String),
    `mandate_data`  Nullable(String),
    `inserted_at` DateTime DEFAULT now() CODEC(T64, LZ4),
    `sign_flag` Int8,
    INDEX connectorIndex connector TYPE bloom_filter GRANULARITY 1,
    INDEX paymentMethodIndex payment_method TYPE bloom_filter GRANULARITY 1,
    INDEX authenticationTypeIndex authentication_type TYPE bloom_filter GRANULARITY 1,
    INDEX currencyIndex currency TYPE bloom_filter GRANULARITY 1,
    INDEX statusIndex status TYPE bloom_filter GRANULARITY 1
) ENGINE = CollapsingMergeTree(
    sign_flag
)
PARTITION BY toStartOfDay(created_at)
ORDER BY
    (created_at, merchant_id, attempt_id)
TTL created_at + toIntervalMonth(6)
;


CREATE MATERIALIZED VIEW kafka_parse_pa TO payment_attempt_dist (
    `payment_id` String,
    `merchant_id` String,
    `attempt_id` String,
    `status` LowCardinality(String),
    `amount` Nullable(UInt32),
    `currency` LowCardinality(Nullable(String)),
    `connector` LowCardinality(Nullable(String)),
    `save_to_locker` Nullable(Bool),
    `error_message` Nullable(String),
    `offer_amount` Nullable(UInt32),
    `surcharge_amount` Nullable(UInt32),
    `tax_amount` Nullable(UInt32),
    `payment_method_id` Nullable(String),
    `payment_method` LowCardinality(Nullable(String)),
    `payment_method_type` LowCardinality(Nullable(String)),
    `connector_transaction_id` Nullable(String),
    `capture_method` Nullable(String),
    `confirm` Bool,
    `authentication_type` LowCardinality(Nullable(String)),
    `cancellation_reason` Nullable(String),
    `amount_to_capture` Nullable(UInt32),
    `mandate_id` Nullable(String),
    `browser_info` Nullable(String),
    `error_code` Nullable(String),
    `connector_metadata` Nullable(String),
    `payment_experience` Nullable(String),
    `created_at` DateTime64(3),
    `capture_on` Nullable(DateTime64(3)),
    `last_synced` Nullable(DateTime64(3)),
    `modified_at` DateTime64(3),
    `payment_method_data` Nullable(String),
    `error_reason` Nullable(String),
    `multiple_capture_count` Nullable(Int16),
    `amount_capturable` Nullable(UInt64) ,
    `merchant_connector_id`  Nullable(String),
    `net_amount` Nullable(UInt64) ,
    `unified_code`  Nullable(String),
    `unified_message`  Nullable(String),
    `mandate_data`  Nullable(String),
    `inserted_at` DateTime64(3),
    `sign_flag` Int8
) AS
SELECT
    payment_id,
    merchant_id,
    attempt_id,
    status,
    amount,
    currency,
    connector,
    save_to_locker,
    error_message,
    offer_amount,
    surcharge_amount,
    tax_amount,
    payment_method_id,
    payment_method,
    payment_method_type,
    connector_transaction_id,
    capture_method,
    confirm,
    authentication_type,
    cancellation_reason,
    amount_to_capture,
    mandate_id,
    browser_info,
    error_code,
    connector_metadata,
    payment_experience,
    created_at,
    capture_on,
    last_synced,
    modified_at,
    payment_method_data,
    error_reason,
    multiple_capture_count,
    amount_capturable,
    merchant_connector_id,
    net_amount,
    unified_code,
    unified_message,
    mandate_data,
    now() as inserted_at,
    sign_flag
FROM
    payment_attempts_queue;

