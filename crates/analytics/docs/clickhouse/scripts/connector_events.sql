CREATE TABLE connector_events_queue (
    `merchant_id` String,
    `payment_id` Nullable(String),
    `connector_name` LowCardinality(String),
    `request_id` String,
    `flow` LowCardinality(String),
    `request` String,
    `masked_response` Nullable(String),
    `error` Nullable(String),
    `status_code` UInt32,
    `created_at` DateTime64(3),
    `latency` UInt128,
    `method` LowCardinality(String),
    `dispute_id` Nullable(String),
    `refund_id` Nullable(String)
) ENGINE = Kafka SETTINGS kafka_broker_list = 'kafka0:29092',
kafka_topic_list = 'hyperswitch-outgoing-connector-events',
kafka_group_name = 'hyper',
kafka_format = 'JSONEachRow',
kafka_handle_error_mode = 'stream';

CREATE MATERIALIZED VIEW connector_events_parse_errors (
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
    connector_events_queue
WHERE
    length(_error) > 0;

CREATE TABLE connector_events (
    `merchant_id` LowCardinality(String),
    `payment_id` Nullable(String),
    `connector_name` LowCardinality(String),
    `request_id` String,
    `flow` LowCardinality(String),
    `request` String,
    `response` Nullable(String),
    `masked_response` Nullable(String),
    `error` Nullable(String),
    `status_code` UInt32,
    `created_at` DateTime64(3),
    `inserted_at` DateTime DEFAULT now() CODEC(T64, LZ4),
    `latency` UInt128,
    `method` LowCardinality(String),
    `dispute_id` Nullable(String),
    `refund_id` Nullable(String),
    INDEX flowIndex flow TYPE bloom_filter GRANULARITY 1,
    INDEX connectorIndex connector_name TYPE bloom_filter GRANULARITY 1,
    INDEX statusIndex status_code TYPE bloom_filter GRANULARITY 1
) ENGINE = MergeTree PARTITION BY toStartOfDay(created_at)
ORDER BY
    (
        created_at,
        merchant_id,
        connector_name,
        flow,
        status_code
    ) TTL inserted_at + toIntervalMonth(18) SETTINGS index_granularity = 8192;

CREATE TABLE connector_events_audit (
    `merchant_id` LowCardinality(String),
    `payment_id` String,
    `connector_name` LowCardinality(String),
    `request_id` String,
    `flow` LowCardinality(String),
    `request` String,
    `response` Nullable(String),
    `masked_response` Nullable(String),
    `error` Nullable(String),
    `status_code` UInt32,
    `created_at` DateTime64(3),
    `inserted_at` DateTime DEFAULT now() CODEC(T64, LZ4),
    `latency` UInt128,
    `method` LowCardinality(String),
    `dispute_id` Nullable(String),
    `refund_id` Nullable(String),
    INDEX flowIndex flow TYPE bloom_filter GRANULARITY 1,
    INDEX connectorIndex connector_name TYPE bloom_filter GRANULARITY 1,
    INDEX statusIndex status_code TYPE bloom_filter GRANULARITY 1
) ENGINE = MergeTree PARTITION BY merchant_id
ORDER BY
    (merchant_id, payment_id) TTL inserted_at + toIntervalMonth(18) SETTINGS index_granularity = 8192;

CREATE MATERIALIZED VIEW connector_events_audit_mv TO connector_events_audit (
    `merchant_id` String,
    `payment_id` Nullable(String),
    `connector_name` LowCardinality(String),
    `request_id` String,
    `flow` LowCardinality(String),
    `request` String,
    `response` Nullable(String),
    `masked_response` Nullable(String),
    `error` Nullable(String),
    `status_code` UInt32,
    `created_at` DateTime64(3),
    `inserted_at` DateTime DEFAULT now() CODEC(T64, LZ4),
    `latency` UInt128,
    `method` LowCardinality(String),
    `refund_id` Nullable(String),
    `dispute_id` Nullable(String)
) AS
SELECT
    merchant_id,
    payment_id,
    connector_name,
    request_id,
    flow,
    request,
    masked_response AS response,
    masked_response,
    error,
    status_code,
    created_at,
    now64() AS inserted_at,
    latency,
    method,
    refund_id,
    dispute_id
FROM
    connector_events_queue
WHERE
    (length(_error) = 0)
    AND (payment_id IS NOT NULL);

CREATE MATERIALIZED VIEW connector_events_mv TO connector_events (
    `merchant_id` String,
    `payment_id` Nullable(String),
    `connector_name` LowCardinality(String),
    `request_id` String,
    `flow` LowCardinality(String),
    `request` String,
    `response` Nullable(String),
    `masked_response` Nullable(String),
    `error` Nullable(String),
    `status_code` UInt32,
    `created_at` DateTime64(3),
    `inserted_at` DateTime DEFAULT now() CODEC(T64, LZ4),
    `latency` UInt128,
    `method` LowCardinality(String),
    `refund_id` Nullable(String),
    `dispute_id` Nullable(String)
) AS
SELECT
    merchant_id,
    payment_id,
    connector_name,
    request_id,
    flow,
    request,
    masked_response AS response,
    masked_response,
    error,
    status_code,
    created_at,
    now64() AS inserted_at,
    latency,
    method,
    refund_id,
    dispute_id
FROM
    connector_events_queue
WHERE
    length(_error) = 0;