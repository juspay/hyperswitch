CREATE TABLE microservice_events_queue (
    `tenant_id` String,
    `service_name` LowCardinality(String),
    `flow` LowCardinality(String),
    `request` Nullable(String),
    `masked_response` Nullable(String),
    `error` Nullable(String),
    `url` String,
    `method` LowCardinality(String),
    `merchant_id` Nullable(String),
    `profile_id` Nullable(String),
    `created_at` DateTime64(3),
    `request_id` String,
    `latency` UInt128,
    `status_code` Nullable(UInt32)
) ENGINE = Kafka SETTINGS kafka_broker_list = 'kafka0:29092',
kafka_topic_list = 'hyperswitch-microservice-api-log-events',
kafka_group_name = 'hyper',
kafka_format = 'JSONEachRow',
kafka_handle_error_mode = 'stream';

CREATE TABLE microservice_events (
    `tenant_id` String,
    `service_name` LowCardinality(String),
    `flow` LowCardinality(String),
    `request` Nullable(String),
    `masked_response` Nullable(String),
    `error` Nullable(String),
    `url` String,
    `method` LowCardinality(String),
    `merchant_id` Nullable(String),
    `profile_id` Nullable(String),
    `created_at` DateTime64(3),
    `inserted_at` DateTime DEFAULT now() CODEC(T64, LZ4),
    `request_id` String,
    `latency` UInt128,
    `status_code` Nullable(UInt32),
    INDEX serviceIndex service_name TYPE bloom_filter GRANULARITY 1,
    INDEX flowIndex flow TYPE bloom_filter GRANULARITY 1,
    INDEX requestIdIndex request_id TYPE bloom_filter GRANULARITY 1
) ENGINE = MergeTree PARTITION BY toStartOfDay(created_at)
ORDER BY
    (
        created_at,
        service_name,
        flow,
        status_code
    ) TTL inserted_at + toIntervalMonth(18) SETTINGS index_granularity = 8192;

CREATE MATERIALIZED VIEW microservice_events_mv TO microservice_events (
    `tenant_id` String,
    `service_name` LowCardinality(String),
    `flow` LowCardinality(String),
    `request` Nullable(String),
    `masked_response` Nullable(String),
    `error` Nullable(String),
    `url` String,
    `method` LowCardinality(String),
    `merchant_id` Nullable(String),
    `profile_id` Nullable(String),
    `created_at` DateTime64(3),
    `inserted_at` DateTime64(3),
    `request_id` String,
    `latency` UInt128,
    `status_code` Nullable(UInt32)
) AS
SELECT
    tenant_id,
    service_name,
    flow,
    request,
    masked_response,
    error,
    url,
    method,
    merchant_id,
    profile_id,
    created_at,
    now() AS inserted_at,
    request_id,
    latency,
    status_code
FROM
    microservice_events_queue
WHERE
    length(_error) = 0;

CREATE MATERIALIZED VIEW microservice_events_parse_errors (
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
    microservice_events_queue
WHERE
    length(_error) > 0;
