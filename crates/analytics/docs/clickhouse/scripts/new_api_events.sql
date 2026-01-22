-- New API Events Queue for Test Extension Middleware
-- This queue reads from a dedicated Kafka topic for NewApiEvent
-- Reuses the same event handler, kafka producer with a new topic (hyperswitch-new-api-log-events)

-- Queue table for NewApiEvent messages
-- Uses stream error mode to handle messages that don't match the original api_events_queue schema
CREATE TABLE new_api_events_queue (
    `tenant_id` Nullable(String),
    `merchant_id` Nullable(String),
    `api_flow` Nullable(String),
    `request_id` Nullable(String),
    `flow_type` Nullable(String),
    `status_code` Int64,
    `api_auth_type` Nullable(String),
    `request` Nullable(String),
    `response` Nullable(String),
    `error` Nullable(String),
    `user_agent` Nullable(String),
    `ip_addr` Nullable(String),
    `url_path` Nullable(String),
    `http_method` Nullable(String),
    `latency` UInt128,
    `hs_latency` Nullable(UInt128),
    `created_at_timestamp` DateTime64(3),
) ENGINE = Kafka SETTINGS kafka_broker_list = 'kafka0:29092',
kafka_topic_list = 'hyperswitch-new-api-log-events',
kafka_group_name = 'hyper_new_api_events',
kafka_format = 'JSONEachRow',
kafka_handle_error_mode = 'stream';

-- Parse errors table for new_api_events_queue
CREATE MATERIALIZED VIEW new_api_events_parse_errors (
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
    new_api_events_queue
WHERE
    length(_error) > 0;

-- Final table to store NewApiEvent from TestExtensionMiddleware
CREATE TABLE new_api_events (
    `tenant_id` Nullable(String),
    `merchant_id` Nullable(String),
    `api_flow` Nullable(String),
    `request_id` Nullable(String),
    `flow_type` Nullable(String),
    `status_code` UInt32,
    `api_auth_type` Nullable(String),
    `request` Nullable(String),
    `response` Nullable(String),
    `error` Nullable(String),
    `user_agent` Nullable(String),
    `ip_addr` Nullable(String),
    `url_path` Nullable(String),
    `http_method` Nullable(String),
    `latency` UInt128,
    `hs_latency` Nullable(UInt128),
    `created_at` DateTime64(3),
    `inserted_at` DateTime DEFAULT now() CODEC(T64, LZ4),
    INDEX statusIndex status_code TYPE bloom_filter GRANULARITY 1,
    INDEX flowIndex api_flow TYPE bloom_filter GRANULARITY 1,
    INDEX merchantIndex merchant_id TYPE bloom_filter GRANULARITY 1
) ENGINE = MergeTree 
PARTITION BY toStartOfDay(created_at)
ORDER BY (created_at, status_code)
TTL inserted_at + toIntervalMonth(6) 
SETTINGS index_granularity = 8192;

-- Materialized View to insert valid NewApiEvent messages from the queue
CREATE MATERIALIZED VIEW new_api_events_mv TO new_api_events AS
SELECT
    tenant_id,
    merchant_id,
    api_flow,
    request_id,
    flow_type,
    status_code,
    api_auth_type,
    request,
    response,
    error,
    user_agent,
    ip_addr,
    url_path,
    http_method,
    latency,
    hs_latency,
    created_at_timestamp AS created_at,
    now() AS inserted_at
FROM new_api_events_queue
WHERE length(_error) = 0;
