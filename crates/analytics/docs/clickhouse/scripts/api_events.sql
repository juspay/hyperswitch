CREATE TABLE api_events_queue (
    `merchant_id` String,
    `payment_id` Nullable(String),
    `refund_id` Nullable(String),
    `payment_method_id` Nullable(String),
    `payment_method` Nullable(String),
    `payment_method_type` Nullable(String),
    `customer_id` Nullable(String),
    `user_id` Nullable(String),
    `request_id` String,
    `flow_type` LowCardinality(String),
    `api_name` LowCardinality(String),
    `request` String,
    `response` String,
    `status_code` UInt32,
    `created_at` DateTime CODEC(T64, LZ4),
    `latency` Nullable(UInt128),
    `user_agent` Nullable(String),
    `ip_addr` Nullable(String)
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
    `request_id` Nullable(String),
    `flow_type` LowCardinality(String),
    `api_name` LowCardinality(String),
    `request` String,
    `response` String,
    `status_code` UInt32,
    `inserted_at` DateTime DEFAULT now() CODEC(T64, LZ4),
    `created_at` DateTime DEFAULT now() CODEC(T64, LZ4),
    `latency` Nullable(UInt128),
    `user_agent` Nullable(String),
    `ip_addr` Nullable(String),
    INDEX flowIndex flow_type TYPE bloom_filter GRANULARITY 1,
    INDEX apiIndex api_name TYPE bloom_filter GRANULARITY 1,
    INDEX statusIndex status_code TYPE bloom_filter GRANULARITY 1
) ENGINE = MergeTree
PARTITION BY toStartOfDay(created_at)
ORDER BY
	(created_at, merchant_id, flow_type, status_code, api_name)
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
    `request_id` Nullable(String),
    `flow_type` LowCardinality(String),
    `api_name` LowCardinality(String),
    `request` String,
    `response` String,
    `status_code` UInt32,
    `inserted_at` DateTime64(3),
    `created_at` DateTime64(3),
    `latency` Nullable(UInt128),
    `user_agent` Nullable(String),
    `ip_addr` Nullable(String)
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
    request_id,
    flow_type,
    api_name,
    request,
    response,
    status_code,
    now() as inserted_at,
    created_at,
    latency,
    user_agent,
    ip_addr
FROM
    api_events_queue;