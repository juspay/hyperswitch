-- Need to discuss with the team about the structure of this table.
CREATE TABLE routing_events_queue
(
    `profile_id` String,
    `payment_id` String,
    `routable_connectors` Array(String),
    `request_id` String,
    `flow` LowCardinality(String),
    `request` String,
    `masked_response` Nullable(String),
    `error` Nullable(String),
    `status_code` Nullable(UInt32),
    `created_at` DateTime64(3),
    `method` LowCardinality(String),
    `routing_engine` LowCardinality(String),
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
    `profile_id` String,
    `payment_id` String,
    `routable_connectors` Array(String),
    `request_id` String,
    `flow` LowCardinality(String),
    `request` String,
    `response` Nullable(String),
    `masked_response` Nullable(String),
    `error` Nullable(String),
    `status_code` Nullable(UInt32),
    `created_at` DateTime64(3),
    `inserted_at` DateTime DEFAULT now() CODEC(T64, LZ4),
    `method` LowCardinality(String),
    INDEX flowIndex flow TYPE bloom_filter GRANULARITY 1,
    INDEX profileIndex profile_id TYPE bloom_filter GRANULARITY 1
) ENGINE = MergeTree PARTITION BY toStartOfDay(created_at)
ORDER BY
    (
        created_at,
        profile_id,
        flow
    ) TTL inserted_at + toIntervalMonth(18) SETTINGS index_granularity = 8192;


CREATE MATERIALIZED VIEW routing_events_mv TO routing_events (
    `profile_id` String,
    `payment_id` String,
    `routable_connectors` Array(String),
    `request_id` String,
    `flow` LowCardinality(String),
    `request` String,
    `response` Nullable(String),
    `masked_response` Nullable(String),
    `error` Nullable(String),
    `status_code` Nullable(UInt32),
    `created_at` DateTime64(3),
    `inserted_at` DateTime DEFAULT now() CODEC(T64, LZ4),
    `method` LowCardinality(String)
) AS
SELECT
    profile_id,
    payment_id,
    routable_connectors,
    request_id,
    flow,
    request,
    masked_response AS response,
    masked_response,
    error,
    status_code,
    created_at,
    now64() AS inserted_at,
    method
FROM
    routing_events_queue
WHERE
    length(_error) = 0;