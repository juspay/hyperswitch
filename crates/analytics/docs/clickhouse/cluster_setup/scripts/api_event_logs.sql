CREATE TABLE hyperswitch.api_events_queue on cluster '{cluster}' (
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
    `url_path` LowCardinality(Nullable(String)),
    `event_type` LowCardinality(Nullable(String)),
    `created_at` DateTime CODEC(T64, LZ4),
    `latency` Nullable(UInt128),
    `user_agent` Nullable(String),
    `ip_addr` Nullable(String),
    `dispute_id` Nullable(String)
) ENGINE = Kafka SETTINGS kafka_broker_list = 'hyper-c1-kafka-brokers.kafka-cluster.svc.cluster.local:9092',
kafka_topic_list = 'hyperswitch-api-log-events',
kafka_group_name = 'hyper-c1',
kafka_format = 'JSONEachRow',
kafka_handle_error_mode = 'stream';


CREATE TABLE hyperswitch.api_events_clustered on cluster '{cluster}' (
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
) ENGINE = ReplicatedMergeTree(
    '/clickhouse/{installation}/{cluster}/tables/{shard}/hyperswitch/api_events_clustered',
    '{replica}'
)
PARTITION BY toStartOfDay(created_at)
ORDER BY
	(created_at, merchant_id, flow_type, status_code, api_name)
TTL created_at + toIntervalMonth(6)
;


CREATE TABLE hyperswitch.api_events_dist on cluster '{cluster}' (
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
    `url_path` LowCardinality(Nullable(String)),
    `event_type` LowCardinality(Nullable(String)),
    `inserted_at` DateTime64(3),
    `created_at` DateTime64(3),
    `latency` Nullable(UInt128),
    `user_agent` Nullable(String),
    `ip_addr` Nullable(String),
    `dispute_id` Nullable(String)
) ENGINE = Distributed('{cluster}', 'hyperswitch', 'api_events_clustered', rand());

CREATE MATERIALIZED VIEW hyperswitch.api_events_mv on cluster '{cluster}' TO hyperswitch.api_events_dist (
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
    `url_path` LowCardinality(Nullable(String)),
    `event_type` LowCardinality(Nullable(String)),
    `inserted_at` DateTime64(3),
    `created_at` DateTime64(3),
    `latency` Nullable(UInt128),
    `user_agent` Nullable(String),
    `ip_addr` Nullable(String),
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
    request_id,
    flow_type,
    api_name,
    request,
    response,
    status_code,
    url_path,
    event_type,
    now() as inserted_at,
    created_at,
    latency,
    user_agent,
    ip_addr
FROM
    hyperswitch.api_events_queue
WHERE length(_error) = 0;


CREATE MATERIALIZED VIEW hyperswitch.api_events_parse_errors on cluster '{cluster}'
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
FROM hyperswitch.api_events_queue
WHERE length(_error) > 0
;


ALTER TABLE hyperswitch.api_events_clustered on cluster '{cluster}' ADD COLUMN `url_path` LowCardinality(Nullable(String));
ALTER TABLE hyperswitch.api_events_clustered on cluster '{cluster}' ADD COLUMN `event_type` LowCardinality(Nullable(String));
ALTER TABLE hyperswitch.api_events_clustered on cluster '{cluster}' ADD COLUMN `dispute_id` Nullable(String);

CREATE TABLE hyperswitch.api_audit_log ON CLUSTER '{cluster}' (
    `merchant_id` LowCardinality(String),
    `payment_id` String,
    `refund_id` Nullable(String),
    `payment_method_id` Nullable(String),
    `payment_method` Nullable(String),
    `payment_method_type` Nullable(String),
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
    `url_path` LowCardinality(Nullable(String)),
    `event_type` LowCardinality(Nullable(String)),
    `customer_id` LowCardinality(Nullable(String))
) ENGINE = ReplicatedMergeTree( '/clickhouse/{installation}/{cluster}/tables/{shard}/hyperswitch/api_audit_log', '{replica}' ) PARTITION BY merchant_id
ORDER BY (merchant_id, payment_id) 
TTL created_at + toIntervalMonth(18) 
SETTINGS index_granularity = 8192


CREATE MATERIALIZED VIEW hyperswitch.api_audit_log_mv ON CLUSTER `{cluster}` TO hyperswitch.api_audit_log(
    `merchant_id` LowCardinality(String),
    `payment_id` String,
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
    `url_path` LowCardinality(Nullable(String)),
    `event_type` LowCardinality(Nullable(String)),
    `inserted_at` DateTime64(3),
    `created_at` DateTime64(3),
    `latency` Nullable(UInt128),
    `user_agent` Nullable(String),
    `ip_addr` Nullable(String),
    `dispute_id` Nullable(String)
) AS 
SELECT 
    merchant_id,
    multiIf(payment_id IS NULL, '', payment_id) AS payment_id,
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
    url_path,
    api_event_type AS event_type,
    now() AS inserted_at,
    created_at,
    latency,
    user_agent,
    ip_addr,
    dispute_id
FROM hyperswitch.api_events_queue
WHERE length(_error) = 0