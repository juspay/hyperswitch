CREATE TABLE account_updater_events_queue
(
    `request_id` Nullable(String),
    `merchant_id` String,
    `profile_id` String,
    `payment_method_id` String,
    `card_network` LowCardinality(Nullable(String)),
    `updater_outcome` LowCardinality(Nullable(String)),
    `error_category` LowCardinality(Nullable(String)),
    `latency_ms` UInt64,
    `created_at` DateTime64(9)
)
ENGINE = Kafka
SETTINGS kafka_broker_list = 'kafka0:29092', kafka_topic_list = 'hyperswitch-account-updater-events', kafka_group_name = 'hyper', kafka_format = 'JSONEachRow', kafka_handle_error_mode = 'stream';

CREATE MATERIALIZED VIEW account_updater_events_parse_errors (
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
    account_updater_events_queue
WHERE
    length(_error) > 0;

CREATE TABLE account_updater_events (
    `request_id` Nullable(String),
    `merchant_id` String,
    `profile_id` String,
    `payment_method_id` String,
    `card_network` LowCardinality(Nullable(String)),
    `updater_outcome` LowCardinality(Nullable(String)),
    `error_category` LowCardinality(Nullable(String)),
    `latency_ms` UInt64,
    `created_at` DateTime64(9),
    `inserted_at` DateTime DEFAULT now() CODEC(T64, LZ4),
    INDEX paymentMethodIndex payment_method_id TYPE bloom_filter GRANULARITY 1,
    INDEX profileIndex profile_id TYPE bloom_filter GRANULARITY 1
) ENGINE = MergeTree
PARTITION BY toStartOfDay(created_at)
ORDER BY ( created_at, merchant_id, profile_id, payment_method_id )
SETTINGS index_granularity = 8192;

CREATE MATERIALIZED VIEW account_updater_events_mv TO account_updater_events (
    `request_id` Nullable(String),
    `merchant_id` String,
    `profile_id` String,
    `payment_method_id` String,
    `card_network` LowCardinality(Nullable(String)),
    `updater_outcome` LowCardinality(Nullable(String)),
    `error_category` LowCardinality(Nullable(String)),
    `latency_ms` UInt64,
    `created_at` DateTime64(9),
    `inserted_at` DateTime DEFAULT now() CODEC(T64, LZ4)
) AS
SELECT
    request_id,
    merchant_id,
    profile_id,
    payment_method_id,
    card_network,
    updater_outcome,
    error_category,
    latency_ms,
    created_at,
    now() AS inserted_at
FROM
    account_updater_events_queue
WHERE
    length(_error) = 0;
