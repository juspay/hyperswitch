CREATE TABLE sdk_events_queue ( 
    `payment_id` Nullable(String),
    `merchant_id` String,
    `remote_ip` Nullable(String),
    `log_type` LowCardinality(Nullable(String)),
    `event_name` LowCardinality(Nullable(String)),
    `first_event` LowCardinality(Nullable(String)),
    `latency` Nullable(UInt32),
    `timestamp` DateTime64(3),
    `browser_name` LowCardinality(Nullable(String)),
    `browser_version` Nullable(String),
    `platform` LowCardinality(Nullable(String)),
    `source` LowCardinality(Nullable(String)),
    `category` LowCardinality(Nullable(String)),
    `version` LowCardinality(Nullable(String)),
    `value` Nullable(String),
    `component` LowCardinality(Nullable(String)),
    `payment_method` LowCardinality(Nullable(String)),
    `payment_experience` LowCardinality(Nullable(String))
) ENGINE = Kafka SETTINGS
    kafka_broker_list = 'kafka0:29092', 
    kafka_topic_list = 'hyper-sdk-logs', 
    kafka_group_name = 'hyper-ckh', 
    kafka_format = 'JSONEachRow', 
    kafka_handle_error_mode = 'stream';

CREATE TABLE sdk_events ( 
    `payment_id` Nullable(String),
    `merchant_id` String,
    `remote_ip` Nullable(String),
    `log_type` LowCardinality(Nullable(String)),
    `event_name` LowCardinality(Nullable(String)),
    `first_event` Bool DEFAULT 1,
    `browser_name` LowCardinality(Nullable(String)),
    `browser_version` Nullable(String),
    `platform` LowCardinality(Nullable(String)),
    `source` LowCardinality(Nullable(String)),
    `category` LowCardinality(Nullable(String)),
    `version` LowCardinality(Nullable(String)),
    `component` LowCardinality(Nullable(String)),
    `payment_method` LowCardinality(Nullable(String)),
    `payment_experience` LowCardinality(Nullable(String)) DEFAULT '',
    `created_at` DateTime DEFAULT now() CODEC(T64, LZ4),
    `inserted_at` DateTime DEFAULT now() CODEC(T64, LZ4),
    `latency` Nullable(UInt32) DEFAULT 0,
    `value` Nullable(String),
    `created_at_precise` DateTime64(3),
    INDEX paymentMethodIndex payment_method TYPE bloom_filter GRANULARITY 1,
    INDEX eventIndex event_name TYPE bloom_filter GRANULARITY 1,
    INDEX platformIndex platform TYPE bloom_filter GRANULARITY 1,
    INDEX logTypeIndex log_type TYPE bloom_filter GRANULARITY 1,
    INDEX categoryIndex category TYPE bloom_filter GRANULARITY 1,
    INDEX sourceIndex source TYPE bloom_filter GRANULARITY 1,
    INDEX componentIndex component TYPE bloom_filter GRANULARITY 1,
    INDEX firstEventIndex first_event TYPE bloom_filter GRANULARITY 1
) ENGINE = MergeTree
PARTITION BY 
    toStartOfDay(created_at) 
ORDER BY 
    (created_at, merchant_id) 
TTL 
    toDateTime(created_at) + toIntervalMonth(6) 
SETTINGS 
    index_granularity = 8192
;

CREATE MATERIALIZED VIEW sdk_events_mv TO sdk_events ( 
    `payment_id` Nullable(String),
    `merchant_id` String,
    `remote_ip` Nullable(String),
    `log_type` LowCardinality(Nullable(String)),
    `event_name` LowCardinality(Nullable(String)),
    `first_event` Bool,
    `latency` Nullable(UInt32),
    `browser_name` LowCardinality(Nullable(String)),
    `browser_version` Nullable(String),
    `platform` LowCardinality(Nullable(String)),
    `source` LowCardinality(Nullable(String)),
    `category` LowCardinality(Nullable(String)),
    `version` LowCardinality(Nullable(String)),
    `value` Nullable(String),
    `component` LowCardinality(Nullable(String)),
    `payment_method` LowCardinality(Nullable(String)),
    `payment_experience` LowCardinality(Nullable(String)),
    `created_at` DateTime64(3),
    `created_at_precise` DateTime64(3)
) AS 
SELECT
    payment_id,
    merchant_id,
    remote_ip,
    log_type,
    event_name,
    multiIf(first_event = 'true', 1, 0) AS first_event,
    latency,
    browser_name,
    browser_version,
    platform,
    source,
    category,
    version,
    value,
    component,
    payment_method,
    payment_experience,
    toDateTime64(timestamp, 3) AS created_at,
    toDateTime64(timestamp, 3) AS created_at_precise 
FROM 
    sdk_events_queue
WHERE length(_error) = 0;

CREATE TABLE sdk_events_audit (
    `payment_id` String,
    `merchant_id` String,
    `remote_ip` Nullable(String),
    `log_type` LowCardinality(Nullable(String)),
    `event_name` LowCardinality(Nullable(String)),
    `first_event` Bool,
    `latency` Nullable(UInt32),
    `browser_name` LowCardinality(Nullable(String)),
    `browser_version` Nullable(String),
    `platform` LowCardinality(Nullable(String)),
    `source` LowCardinality(Nullable(String)),
    `category` LowCardinality(Nullable(String)),
    `version` LowCardinality(Nullable(String)),
    `value` Nullable(String),
    `component` LowCardinality(Nullable(String)),
    `payment_method` LowCardinality(Nullable(String)),
    `payment_experience` LowCardinality(Nullable(String)),
    `created_at` DateTime DEFAULT now() CODEC(T64, LZ4),
    `created_at_precise` DateTime64(3),
    `inserted_at` DateTime DEFAULT now() CODEC(T64, LZ4)
) ENGINE = MergeTree PARTITION BY merchant_id
ORDER BY
    (merchant_id, payment_id)
    TTL inserted_at + toIntervalMonth(18)
SETTINGS index_granularity = 8192;

CREATE MATERIALIZED VIEW sdk_events_parse_errors (
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
    sdk_events_queue
WHERE
    length(_error) > 0;

CREATE MATERIALIZED VIEW sdk_events_audit_mv TO sdk_events_audit (
    `payment_id` String,
    `merchant_id` String,
    `remote_ip` Nullable(String),
    `log_type` LowCardinality(Nullable(String)),
    `event_name` LowCardinality(Nullable(String)),
    `first_event` Bool,
    `latency` Nullable(UInt32),
    `browser_name` LowCardinality(Nullable(String)),
    `browser_version` Nullable(String),
    `platform` LowCardinality(Nullable(String)),
    `source` LowCardinality(Nullable(String)),
    `category` LowCardinality(Nullable(String)),
    `version` LowCardinality(Nullable(String)),
    `value` Nullable(String),
    `component` LowCardinality(Nullable(String)),
    `payment_method` LowCardinality(Nullable(String)),
    `payment_experience` LowCardinality(Nullable(String)),
    `created_at` DateTime64(3),
    `created_at_precise` DateTime64(3),
    `inserted_at` DateTime DEFAULT now() CODEC(T64, LZ4)
) AS
SELECT
    payment_id,
    merchant_id,
    remote_ip,
    log_type,
    event_name,
    multiIf(first_event = 'true', 1, 0) AS first_event,
    latency,
    browser_name,
    browser_version,
    platform,
    source,
    category,
    version,
    value,
    component,
    payment_method,
    payment_experience,
    toDateTime64(timestamp, 3) AS created_at,
    toDateTime64(timestamp, 3) AS created_at_precise,
    now() AS inserted_at
FROM
    sdk_events_queue
WHERE
    (length(_error) = 0)
    AND (payment_id IS NOT NULL);

CREATE TABLE active_payments ( 
    `payment_id` Nullable(String),
    `merchant_id` String,
    `created_at` DateTime64,
    `flow_type` LowCardinality(Nullable(String)),
    INDEX merchantIndex merchant_id TYPE bloom_filter GRANULARITY 1,
    INDEX flowTypeIndex flow_type TYPE bloom_filter GRANULARITY 1
) ENGINE = MergeTree
PARTITION BY toStartOfSecond(created_at)
ORDER BY 
    merchant_id
TTL 
    toDateTime(created_at) + INTERVAL 60 SECOND
SETTINGS 
    index_granularity = 8192;

CREATE MATERIALIZED VIEW sdk_active_payments_mv TO active_payments ( 
    `payment_id` Nullable(String),
    `merchant_id` String,
    `created_at` DateTime64,
    `flow_type` LowCardinality(Nullable(String))
) AS 
SELECT
    payment_id,
    merchant_id,
    toDateTime64(timestamp, 3) AS created_at,
    'sdk' AS flow_type
FROM 
    sdk_events_queue
WHERE length(_error) = 0;

CREATE MATERIALIZED VIEW api_active_payments_mv TO active_payments ( 
    `payment_id` Nullable(String),
    `merchant_id` String,
    `created_at` DateTime64,
    `flow_type` LowCardinality(Nullable(String))
) AS 
SELECT
    payment_id,
    merchant_id,
    created_at_timestamp AS created_at,
    flow_type
FROM 
    api_events_queue
WHERE length(_error) = 0;