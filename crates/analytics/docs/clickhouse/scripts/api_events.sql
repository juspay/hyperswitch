CREATE TABLE api_events_queue (
    `merchant_id` String,
    `payment_id` Nullable(String),
    `refund_id` Nullable(String),
    `payment_method_id` Nullable(String),
    `payment_method` Nullable(String),
    `payment_method_type` Nullable(String),
    `customer_id` Nullable(String),
    `user_id` Nullable(String),
    `connector` Nullable(String),
    `request_id` String,
    `flow_type` LowCardinality(String),
    `api_flow` LowCardinality(String),
    `api_auth_type` LowCardinality(String),
    `request` String,
    `response` Nullable(String),
    `error` Nullable(String),
    `authentication_data` Nullable(String),
    `status_code` UInt32,
    `created_at_timestamp` DateTime64(3),
    `latency` UInt128,
    `user_agent` String,
    `ip_addr` String,
    `hs_latency` Nullable(UInt128),
    `http_method` LowCardinality(String),
    `url_path` String,
    `dispute_id` Nullable(String)
) ENGINE = Kafka SETTINGS kafka_broker_list = 'kafka0:29092',
kafka_topic_list = 'hyperswitch-api-log-events',
kafka_group_name = 'hyper-c1',
kafka_format = 'JSONEachRow',
kafka_handle_error_mode = 'stream';


CREATE TABLE api_events_dist (
    `merchant_id` String,
    `payment_id` Nullable(String),
    `refund_id` Nullable(String),
    `payment_method_id` Nullable(String),
    `payment_method` Nullable(String),
    `payment_method_type` Nullable(String),
    `customer_id` Nullable(String),
    `user_id` Nullable(String),
    `connector` Nullable(String),
    `request_id` String,
    `flow_type` LowCardinality(String),
    `api_flow` LowCardinality(String),
    `api_auth_type` LowCardinality(String),
    `request` String,
    `response` Nullable(String),
    `error` Nullable(String),
    `authentication_data` Nullable(String),
    `status_code` UInt32,
    `created_at_timestamp` DateTime64(3),
    `latency` UInt128,
    `user_agent` String,
    `ip_addr` String,
    `hs_latency` Nullable(UInt128),
    `http_method` LowCardinality(String),
    `url_path` String,
    `dispute_id` Nullable(String)
    INDEX flowIndex flow_type TYPE bloom_filter GRANULARITY 1,
    INDEX apiIndex api_flow TYPE bloom_filter GRANULARITY 1,
    INDEX statusIndex status_code TYPE bloom_filter GRANULARITY 1
) ENGINE = MergeTree
PARTITION BY toStartOfDay(created_at)
ORDER BY
	(created_at, merchant_id, flow_type, status_code, api_flow)
TTL created_at + toIntervalMonth(6)
;

CREATE MATERIALIZED VIEW api_events_mv TO api_events_dist (
    `merchant_id` String,
    `payment_id` Nullable(String),
    `refund_id` Nullable(String),
    `payment_method_id` Nullable(String),
    `payment_method` Nullable(String),
    `payment_method_type` Nullable(String),
    `customer_id` Nullable(String),
    `user_id` Nullable(String),
    `connector` Nullable(String),
    `request_id` String,
    `flow_type` LowCardinality(String),
    `api_flow` LowCardinality(String),
    `api_auth_type` LowCardinality(String),
    `request` String,
    `response` Nullable(String),
    `error` Nullable(String),
    `authentication_data` Nullable(String),
    `status_code` UInt32,
    `created_at_timestamp` DateTime64(3),
    `latency` UInt128,
    `user_agent` String,
    `ip_addr` String,
    `hs_latency` Nullable(UInt128),
    `http_method` LowCardinality(String),
    `url_path` String,
    `dispute_id` Nullable(String)
) AS
SELECT
    merchant_id,
    payment_id,
    refund_id,
    payment_method_id,
    payment_method,
    payment_method_type,
    customer_id,
    user_id,
    connector,
    request_id,
    flow_type,
    api_flow,
    api_auth_type,
    request,
    response,
    error,
    authentication_data,
    status_code,
    created_at_timestamp,
    now() as inserted_at,
    latency,
    user_agent,
    ip_addr,
    hs_latency,
    http_method,
    url_path,
    dispute_id
FROM
    api_events_queue
where length(_error) = 0;


CREATE MATERIALIZED VIEW api_events_parse_errors
(
    `topic` String,
    `partition` Int64,
    `offset` Int64,
    `raw` String,
    `error` String
)
ENGINE = MergeTree
ORDER BY (topic, partition, offset)
SETTINGS index_granularity = 8192 AS
SELECT
    _topic AS topic,
    _partition AS partition,
    _offset AS offset,
    _raw_message AS raw,
    _error AS error
FROM api_events_queue
WHERE length(_error) > 0
;
