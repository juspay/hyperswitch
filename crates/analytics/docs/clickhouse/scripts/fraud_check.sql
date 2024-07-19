CREATE TABLE fraud_check_queue (
    `frm_id` String,
    `payment_id` String,
    `merchant_id` String,
    `attempt_id` String,
    `created_at` DateTime CODEC(T64, LZ4),
    `frm_name` LowCardinality(String),
    `frm_transaction_id` String,
    `frm_transaction_type` LowCardinality(String),
    `frm_status` LowCardinality(String),
    `frm_score` Int32,
    `frm_reason` LowCardinality(String),
    `frm_error` Nullable(String),
    `amount` UInt32,
    `currency` LowCardinality(String),
    `payment_method` LowCardinality(String),
    `payment_method_type` LowCardinality(String),
    `refund_transaction_id` Nullable(String),
    `metadata` Nullable(String),
    `modified_at` DateTime CODEC(T64, LZ4),
    `last_step` LowCardinality(String),
    `payment_capture_method` LowCardinality(String),
    `sign_flag` Int8
) ENGINE = Kafka SETTINGS kafka_broker_list = 'kafka0:29092',
kafka_topic_list = 'hyperswitch-fraud-check-events',
kafka_group_name = 'hyper',
kafka_format = 'JSONEachRow',
kafka_handle_error_mode = 'stream';

CREATE TABLE fraud_check (
    `frm_id` String,
    `payment_id` String,
    `merchant_id` LowCardinality(String),
    `attempt_id` String,
    `created_at` DateTime DEFAULT now() CODEC(T64, LZ4),
    `frm_name` LowCardinality(String),
    `frm_transaction_id` String,
    `frm_transaction_type` LowCardinality(String),
    `frm_status` LowCardinality(String),
    `frm_score` Int32,
    `frm_reason` LowCardinality(String),
    `frm_error` Nullable(String),
    `amount` UInt32,
    `currency` LowCardinality(String),
    `payment_method` LowCardinality(String),
    `payment_method_type` LowCardinality(String),
    `refund_transaction_id` Nullable(String),
    `metadata` Nullable(String),
    `modified_at` DateTime DEFAULT now() CODEC(T64, LZ4),
    `last_step` LowCardinality(String),
    `payment_capture_method` LowCardinality(String),
    `sign_flag` Int8,
    INDEX frmNameIndex frm_name TYPE bloom_filter GRANULARITY 1,
    INDEX frmStatusIndex frm_status TYPE bloom_filter GRANULARITY 1,
    INDEX paymentMethodIndex payment_method TYPE bloom_filter GRANULARITY 1,
    INDEX paymentMethodTypeIndex payment_method_type TYPE bloom_filter GRANULARITY 1,
    INDEX currencyIndex currency TYPE bloom_filter GRANULARITY 1
) ENGINE = CollapsingMergeTree(sign_flag) PARTITION BY toStartOfDay(created_at)
ORDER BY
    (created_at, merchant_id, attempt_id, frm_id) TTL created_at + toIntervalMonth(18) SETTINGS index_granularity = 8192;

CREATE MATERIALIZED VIEW fraud_check_mv TO fraud_check (
    `frm_id` String,
    `payment_id` String,
    `merchant_id` String,
    `attempt_id` String,
    `created_at` DateTime64(3),
    `frm_name` LowCardinality(String),
    `frm_transaction_id` String,
    `frm_transaction_type` LowCardinality(String),
    `frm_status` LowCardinality(String),
    `frm_score` Int32,
    `frm_reason` LowCardinality(String),
    `frm_error` Nullable(String),
    `amount` UInt32,
    `currency` LowCardinality(String),
    `payment_method` LowCardinality(String),
    `payment_method_type` LowCardinality(String),
    `refund_transaction_id` Nullable(String),
    `metadata` Nullable(String),
    `modified_at` DateTime64(3),
    `last_step` LowCardinality(String),
    `payment_capture_method` LowCardinality(String),
    `sign_flag` Int8
) AS
SELECT
    frm_id,
    payment_id,
    merchant_id,
    attempt_id,
    created_at,
    frm_name,
    frm_transaction_id,
    frm_transaction_type,
    frm_status,
    frm_score,
    frm_reason,
    frm_error,
    amount,
    currency,
    payment_method,
    payment_method_type,
    refund_transaction_id,
    metadata,
    modified_at,
    last_step,
    payment_capture_method,
    sign_flag
FROM
    fraud_check_queue
WHERE
    length(_error) = 0;

CREATE MATERIALIZED VIEW fraud_check_parse_errors (
    `topic` String,
    `partition` Int64,
    `offset` Int64,
    `raw` String,
    `error` String
) ENGINE = MergeTree
ORDER BY
    (topic, partition, offset) SETTINGS index_granularity = 8192 AS
SELECT
    _topic AS topic,
    _partition AS partition,
    _offset AS offset,
    _raw_message AS raw,
    _error AS error
FROM
    fraud_check_queue
WHERE
    length(_error) > 0;