-- Need to discuss with the team about the structure of this table.
CREATE TABLE routing_events_queue
(
    `merchant_id` String,
    `profile_id` String,
    `payment_id` String,
    `refund_id` Nullable(String),
    `dispute_id` Nullable(String),
    `routable_connectors` String,
    `payment_connector` Nullable(String),
    `request_id` String,
    `flow` LowCardinality(String),
    `url` Nullable(String),
    `request` String,
    `response` Nullable(String),
    `error` Nullable(String),
    `status_code` Nullable(UInt32),
    `created_at` DateTime64(9),
    `method` LowCardinality(String),
    `routing_engine` LowCardinality(String),
    `routing_approach` Nullable(String)
)
ENGINE = Kafka
SETTINGS kafka_broker_list = 'kafka0:29092', kafka_topic_list = 'hyperswitch-routing-api-events', kafka_group_name = 'hyper', kafka_format = 'JSONEachRow', kafka_handle_error_mode = 'stream';

CREATE MATERIALIZED VIEW routing_events_parse_errors (
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
    routing_events_queue
WHERE
    length(_error) > 0;

CREATE TABLE routing_events (
    `merchant_id` String,
    `profile_id` String,
    `payment_id` String,
    `refund_id` Nullable(String),
    `dispute_id` Nullable(String),
    `routable_connectors` String,
    `payment_connector` Nullable(String),
    `request_id` String,
    `flow` LowCardinality(String),
    `url` Nullable(String),
    `request` String,
    `response` Nullable(String),
    `error` Nullable(String),
    `status_code` Nullable(UInt32),
    `created_at` DateTime64(9),
    `inserted_at` DateTime DEFAULT now() CODEC(T64, LZ4),
    `method` LowCardinality(String),
    `routing_engine` LowCardinality(String),
    `routing_approach` Nullable(String),
    INDEX flowIndex flow TYPE bloom_filter GRANULARITY 1,
    INDEX profileIndex profile_id TYPE bloom_filter GRANULARITY 1
) ENGINE = MergeTree
PARTITION BY toStartOfDay(created_at)
ORDER BY ( created_at, merchant_id, profile_id, payment_id )
SETTINGS index_granularity = 8192;

CREATE TABLE routing_events_audit (
    `merchant_id` String,
    `profile_id` String,
    `payment_id` String,
    `refund_id` Nullable(String),
    `dispute_id` Nullable(String),
    `routable_connectors` String,
    `payment_connector` Nullable(String),
    `request_id` String,
    `flow` LowCardinality(String),
    `url` Nullable(String),
    `request` String,
    `response` Nullable(String),
    `error` Nullable(String),
    `status_code` Nullable(UInt32),
    `created_at` DateTime64(9),
    `inserted_at` DateTime DEFAULT now() CODEC(T64, LZ4),
    `method` LowCardinality(String),
    `routing_engine` LowCardinality(String),
    `routing_approach` Nullable(String),
    INDEX flowIndex flow TYPE bloom_filter GRANULARITY 1,
    INDEX profileIndex profile_id TYPE bloom_filter GRANULARITY 1
) ENGINE = MergeTree 
PARTITION BY (merchant_id)
ORDER BY ( merchant_id, payment_id ) 
SETTINGS index_granularity = 8192;


CREATE MATERIALIZED VIEW routing_events_mv TO routing_events (
    `merchant_id` String,
    `profile_id` String,
    `payment_id` String,
    `refund_id` Nullable(String),
    `dispute_id` Nullable(String),
    `routable_connectors` String,
    `payment_connector` Nullable(String),
    `request_id` String,
    `flow` LowCardinality(String),
    `url` Nullable(String),
    `request` String,
    `response` Nullable(String),
    `error` Nullable(String),
    `status_code` Nullable(UInt32),
    `created_at` DateTime64(9),
    `inserted_at` DateTime DEFAULT now() CODEC(T64, LZ4),
    `method` LowCardinality(String),
    `routing_engine` LowCardinality(String),
    `routing_approach` Nullable(String)
) AS
SELECT
    merchant_id,
    profile_id,
    payment_id,
    refund_id,
    dispute_id,
    routable_connectors,
    payment_connector,
    request_id,
    flow,
    url,
    request,
    response,
    error,
    status_code,
    created_at,
    now() AS inserted_at,
    method,
    routing_engine,
    routing_approach
FROM
    routing_events_queue
WHERE
    length(_error) = 0;

CREATE MATERIALIZED VIEW routing_events_audit_mv TO routing_events_audit (
    `merchant_id` String,
    `profile_id` String,
    `payment_id` String,
    `refund_id` Nullable(String),
    `dispute_id` Nullable(String),
    `routable_connectors` String,
    `payment_connector` Nullable(String),
    `request_id` String,
    `flow` LowCardinality(String),
    `url` Nullable(String),
    `request` String,
    `response` Nullable(String),
    `error` Nullable(String),
    `status_code` Nullable(UInt32),
    `created_at` DateTime64(9),
    `inserted_at` DateTime DEFAULT now() CODEC(T64, LZ4),
    `method` LowCardinality(String),
    `routing_engine` LowCardinality(String),
    `routing_approach` Nullable(String)
) AS
SELECT
    merchant_id,
    profile_id,
    payment_id,
    refund_id,
    dispute_id,
    routable_connectors,
    payment_connector,
    request_id,
    flow,
    url,
    request,
    response,
    error,
    status_code,
    created_at,
    now() AS inserted_at,
    method,
    routing_engine,
    routing_approach
FROM
    routing_events_queue
WHERE
    length(_error) = 0;