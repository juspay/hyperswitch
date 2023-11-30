CREATE TABLE hyperswitch.sdk_events_queue on cluster '{cluster}' ( 
    `payment_id` Nullable(String),
    `merchant_id` String,
    `remote_ip` Nullable(String),
    `log_type` LowCardinality(Nullable(String)),
    `event_name` LowCardinality(Nullable(String)),
    `first_event` LowCardinality(Nullable(String)),
    `latency` Nullable(UInt32),
    `timestamp` String,
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
    kafka_broker_list = 'hyper-c1-kafka-brokers.kafka-cluster.svc.cluster.local:9092', 
    kafka_topic_list = 'hyper-sdk-logs', 
    kafka_group_name = 'hyper-c1', 
    kafka_format = 'JSONEachRow', 
    kafka_handle_error_mode = 'stream';

CREATE TABLE hyperswitch.sdk_events_clustered on cluster '{cluster}' ( 
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
    `value` Nullable(String),
    `component` LowCardinality(Nullable(String)),
    `payment_method` LowCardinality(Nullable(String)),
    `payment_experience` LowCardinality(Nullable(String)) DEFAULT '',
    `created_at` DateTime64(3) DEFAULT now64() CODEC(T64, LZ4),
    `inserted_at` DateTime64(3) DEFAULT now64() CODEC(T64, LZ4),
    `latency` Nullable(UInt32) DEFAULT 0,
    INDEX paymentMethodIndex payment_method TYPE bloom_filter GRANULARITY 1,
    INDEX eventIndex event_name TYPE bloom_filter GRANULARITY 1,
    INDEX platformIndex platform TYPE bloom_filter GRANULARITY 1,
    INDEX logTypeIndex log_type TYPE bloom_filter GRANULARITY 1,
    INDEX categoryIndex category TYPE bloom_filter GRANULARITY 1,
    INDEX sourceIndex source TYPE bloom_filter GRANULARITY 1,
    INDEX componentIndex component TYPE bloom_filter GRANULARITY 1,
    INDEX firstEventIndex first_event TYPE bloom_filter GRANULARITY 1 
) ENGINE = ReplicatedMergeTree(
    '/clickhouse/{installation}/{cluster}/tables/{shard}/hyperswitch/sdk_events_clustered', '{replica}'
) 
PARTITION BY 
    toStartOfDay(created_at) 
ORDER BY 
    (created_at, merchant_id) 
TTL 
    toDateTime(created_at) + toIntervalMonth(6) 
SETTINGS 
    index_granularity = 8192
;

CREATE TABLE hyperswitch.sdk_events_dist on cluster '{cluster}' ( 
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
    `value` Nullable(String),
    `component` LowCardinality(Nullable(String)),
    `payment_method` LowCardinality(Nullable(String)),
    `payment_experience` LowCardinality(Nullable(String)) DEFAULT '',
    `created_at` DateTime64(3) DEFAULT now64() CODEC(T64, LZ4),
    `inserted_at` DateTime64(3) DEFAULT now64() CODEC(T64, LZ4),
    `latency` Nullable(UInt32) DEFAULT 0
) ENGINE = Distributed(
    '{cluster}', 'hyperswitch', 'sdk_events_clustered', rand()
);

CREATE MATERIALIZED VIEW hyperswitch.sdk_events_mv on cluster '{cluster}' TO hyperswitch.sdk_events_dist ( 
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
    `created_at` DateTime64(3)
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
    toDateTime64(timestamp, 3) AS created_at 
FROM 
    hyperswitch.sdk_events_queue
WHERE length(_error) = 0
;

CREATE MATERIALIZED VIEW hyperswitch.sdk_parse_errors on cluster '{cluster}' (
    `topic` String,
    `partition` Int64,
    `offset` Int64,
    `raw` String,
    `error` String 
) ENGINE = MergeTree 
    ORDER BY (topic, partition, offset) 
SETTINGS 
    index_granularity = 8192 AS 
SELECT 
    _topic AS topic,
    _partition AS partition,
    _offset AS offset,
    _raw_message AS raw,
    _error AS error 
FROM 
    hyperswitch.sdk_events_queue 
WHERE 
    length(_error) > 0
;
