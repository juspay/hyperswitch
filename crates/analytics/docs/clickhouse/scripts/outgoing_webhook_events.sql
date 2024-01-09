CREATE TABLE
    outgoing_webhook_events_queue (
        `merchant_id` String,
        `event_id` Nullable(String),
        `event_type` LowCardinality(String),
        `outgoing_webhook_event_type` LowCardinality(String),
        `payment_id` Nullable(String),
        `refund_id` Nullable(String),
        `attempt_id` Nullable(String),
        `dispute_id` Nullable(String),
        `payment_method_id` Nullable(String),
        `mandate_id` Nullable(String),
        `content` Nullable(String),
        `is_error` Bool,
        `error` Nullable(String),
        `created_at_timestamp` DateTime64(3)
    ) ENGINE = Kafka SETTINGS kafka_broker_list = 'kafka0:29092',
    kafka_topic_list = 'hyperswitch-outgoing-webhook-events',
    kafka_group_name = 'hyper-c1',
    kafka_format = 'JSONEachRow',
    kafka_handle_error_mode = 'stream';

CREATE TABLE
    outgoing_webhook_events_cluster (
        `merchant_id` String,
        `event_id` String,
        `event_type` LowCardinality(String),
        `outgoing_webhook_event_type` LowCardinality(String),
        `payment_id` Nullable(String),
        `refund_id` Nullable(String),
        `attempt_id` Nullable(String),
        `dispute_id` Nullable(String),
        `payment_method_id` Nullable(String),
        `mandate_id` Nullable(String),
        `content` Nullable(String),
        `is_error` Bool,
        `error` Nullable(String),
        `created_at_timestamp` DateTime64(3),
        `inserted_at` DateTime DEFAULT now() CODEC(T64, LZ4),
        INDEX eventIndex event_type TYPE bloom_filter GRANULARITY 1,
        INDEX webhookeventIndex outgoing_webhook_event_type TYPE bloom_filter GRANULARITY 1
    ) ENGINE = MergeTree PARTITION BY toStartOfDay(created_at_timestamp)
ORDER BY (
        created_at_timestamp,
        merchant_id,
        event_id,
        event_type,
        outgoing_webhook_event_type
    ) TTL inserted_at + toIntervalMonth(6);

CREATE MATERIALIZED VIEW outgoing_webhook_events_mv TO outgoing_webhook_events_cluster (
    `merchant_id` String,
    `event_id` Nullable(String),
    `event_type` LowCardinality(String),
    `outgoing_webhook_event_type` LowCardinality(String),
    `payment_id` Nullable(String),
    `refund_id` Nullable(String),
    `attempt_id` Nullable(String),
    `dispute_id` Nullable(String),
    `payment_method_id` Nullable(String),
    `mandate_id` Nullable(String),
    `content` Nullable(String),
    `is_error` Bool,
    `error` Nullable(String),
    `created_at_timestamp` DateTime64(3),
    `inserted_at` DateTime DEFAULT now() CODEC(T64, LZ4),
) AS
SELECT
    merchant_id,
    event_id,
    event_type,
    outgoing_webhook_event_type,
    payment_id,
    refund_id,
    attempt_id,
    dispute_id,
    payment_method_id,
    mandate_id,
    content,
    is_error,
    error,
    created_at_timestamp,
    now() AS inserted_at
FROM
    outgoing_webhook_events_queue
where length(_error) = 0;

CREATE MATERIALIZED VIEW outgoing_webhook_parse_errors (
    `topic` String,
    `partition` Int64,
    `offset` Int64,
    `raw` String,
    `error` String
) ENGINE = MergeTree
ORDER BY (
        topic, partition,
        offset
    ) SETTINGS index_granularity = 8192 AS
SELECT
    _topic AS topic,
    _partition AS partition,
    _offset AS
offset
,
    _raw_message AS raw,
    _error AS error
FROM
    outgoing_webhook_events_queue
WHERE length(_error) > 0;