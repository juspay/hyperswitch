CREATE TABLE connector_events_queue
(
    `merchant_id` String,
    `payment_id` Nullable(String),
    `connector_name` LowCardinality(String),
    `request_id` String,
    `flow` LowCardinality(String),
    `request` String,
    `masked_response` Nullable(String),
    `error` Nullable(String),
    `status_code` UInt32,
    `created_at` DateTime64(3),
    `latency` UInt128,
    `method` LowCardinality(String),
    `dispute_id` Nullable(String),
    `refund_id` Nullable(String),
    `payout_id` Nullable(String),
    `destination` LowCardinality(Nullable(String)),
    `execution_mode` LowCardinality(Nullable(String))
)
ENGINE = Kafka
SETTINGS kafka_broker_list = 'kafka0:29092', kafka_topic_list = 'hyperswitch-outgoing-connector-events', kafka_group_name = 'hyper', kafka_format = 'JSONEachRow', kafka_handle_error_mode = 'stream';

CREATE MATERIALIZED VIEW connector_events_parse_errors (
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
    connector_events_queue
WHERE
    length(_error) > 0;

CREATE TABLE connector_events (
    `merchant_id` LowCardinality(String),
    `payment_id` Nullable(String),
    `connector_name` LowCardinality(String),
    `request_id` String,
    `flow` LowCardinality(String),
    `request` String,
    `response` Nullable(String),
    `masked_response` Nullable(String),
    `error` Nullable(String),
    `status_code` UInt32,
    `created_at` DateTime64(3),
    `inserted_at` DateTime DEFAULT now() CODEC(T64, LZ4),
    `latency` UInt128,
    `method` LowCardinality(String),
    `dispute_id` Nullable(String),
    `refund_id` Nullable(String),
    `payout_id` Nullable(String),
    `destination` LowCardinality(Nullable(String)),
    `execution_mode` LowCardinality(Nullable(String)),
    `source` LowCardinality(Nullable(String)),
    INDEX flowIndex flow TYPE bloom_filter GRANULARITY 1,
    INDEX connectorIndex connector_name TYPE bloom_filter GRANULARITY 1,
    INDEX statusIndex status_code TYPE bloom_filter GRANULARITY 1
) ENGINE = MergeTree PARTITION BY toStartOfDay(created_at)
ORDER BY
    (
        created_at,
        merchant_id,
        connector_name,
        flow,
        status_code
    ) TTL inserted_at + toIntervalMonth(18) SETTINGS index_granularity = 8192;

CREATE TABLE connector_events_audit (
    `merchant_id` LowCardinality(String),
    `payment_id` String,
    `connector_name` LowCardinality(String),
    `request_id` String,
    `flow` LowCardinality(String),
    `request` String,
    `response` Nullable(String),
    `masked_response` Nullable(String),
    `error` Nullable(String),
    `status_code` UInt32,
    `created_at` DateTime64(3),
    `inserted_at` DateTime DEFAULT now() CODEC(T64, LZ4),
    `latency` UInt128,
    `method` LowCardinality(String),
    `dispute_id` Nullable(String),
    `refund_id` Nullable(String),
    `destination` LowCardinality(Nullable(String)),
    `execution_mode` LowCardinality(Nullable(String)),
    `source` LowCardinality(Nullable(String)),
    INDEX flowIndex flow TYPE bloom_filter GRANULARITY 1,
    INDEX connectorIndex connector_name TYPE bloom_filter GRANULARITY 1,
    INDEX statusIndex status_code TYPE bloom_filter GRANULARITY 1
) ENGINE = MergeTree PARTITION BY merchant_id
ORDER BY
    (merchant_id, payment_id) TTL inserted_at + toIntervalMonth(18) SETTINGS index_granularity = 8192;

CREATE TABLE connector_events_payout_audit (
    `merchant_id` LowCardinality(String),
    `payout_id` String,
    `connector_name` LowCardinality(String),
    `request_id` String,
    `flow` LowCardinality(String),
    `request` String,
    `response` Nullable(String),
    `masked_response` Nullable(String),
    `error` Nullable(String),
    `status_code` UInt32,
    `created_at` DateTime64(3),
    `inserted_at` DateTime DEFAULT now() CODEC(T64, LZ4),
    `latency` UInt128,
    `method` LowCardinality(String),
    `destination` LowCardinality(Nullable(String)),
    `execution_mode` LowCardinality(Nullable(String)),
    `source` LowCardinality(Nullable(String)),
    INDEX flowIndex flow TYPE bloom_filter GRANULARITY 1,
    INDEX connectorIndex connector_name TYPE bloom_filter GRANULARITY 1,
    INDEX statusIndex status_code TYPE bloom_filter GRANULARITY 1
) ENGINE = MergeTree PARTITION BY merchant_id
ORDER BY
    (merchant_id, payout_id) TTL inserted_at + toIntervalMonth(18) SETTINGS index_granularity = 8192;

CREATE MATERIALIZED VIEW connector_events_audit_mv TO connector_events_audit (
    `merchant_id` String,
    `payment_id` Nullable(String),
    `connector_name` LowCardinality(String),
    `request_id` String,
    `flow` LowCardinality(String),
    `request` String,
    `response` Nullable(String),
    `masked_response` Nullable(String),
    `error` Nullable(String),
    `status_code` UInt32,
    `created_at` DateTime64(3),
    `inserted_at` DateTime DEFAULT now() CODEC(T64, LZ4),
    `latency` UInt128,
    `method` LowCardinality(String),
    `refund_id` Nullable(String),
    `dispute_id` Nullable(String),
    `destination` LowCardinality(Nullable(String)),
    `execution_mode` LowCardinality(Nullable(String)),
    `source` LowCardinality(Nullable(String))
) AS
SELECT
    merchant_id,
    payment_id,
    connector_name,
    request_id,
    flow,
    request,
    masked_response AS response,
    masked_response,
    error,
    status_code,
    created_at,
    now64() AS inserted_at,
    latency,
    method,
    refund_id,
    dispute_id,
    destination,
    execution_mode,
    'hyperswitch' AS source
FROM
    connector_events_queue
WHERE
    (length(_error) = 0)
    AND (payment_id IS NOT NULL);

CREATE MATERIALIZED VIEW connector_events_payout_audit_mv TO connector_events_payout_audit (
    `merchant_id` String,
    `payout_id` Nullable(String),
    `connector_name` LowCardinality(String),
    `request_id` String,
    `flow` LowCardinality(String),
    `request` String,
    `response` Nullable(String),
    `masked_response` Nullable(String),
    `error` Nullable(String),
    `status_code` UInt32,
    `created_at` DateTime64(3),
    `inserted_at` DateTime DEFAULT now() CODEC(T64, LZ4),
    `latency` UInt128,
    `method` LowCardinality(String),
    `destination` LowCardinality(Nullable(String)),
    `execution_mode` LowCardinality(Nullable(String)),
    `source` LowCardinality(Nullable(String))
) AS
SELECT
    merchant_id,
    payout_id,
    connector_name,
    request_id,
    flow,
    request,
    masked_response AS response,
    masked_response,
    error,
    status_code,
    created_at,
    now64() AS inserted_at,
    latency,
    method,
    destination,
    execution_mode,
    'hyperswitch' AS source
FROM
    connector_events_queue
WHERE
    (length(_error) = 0)
    AND (payout_id IS NOT NULL);

CREATE MATERIALIZED VIEW connector_events_mv TO connector_events (
    `merchant_id` String,
    `payment_id` Nullable(String),
    `connector_name` LowCardinality(String),
    `request_id` String,
    `flow` LowCardinality(String),
    `request` String,
    `response` Nullable(String),
    `masked_response` Nullable(String),
    `error` Nullable(String),
    `status_code` UInt32,
    `created_at` DateTime64(3),
    `inserted_at` DateTime DEFAULT now() CODEC(T64, LZ4),
    `latency` UInt128,
    `method` LowCardinality(String),
    `refund_id` Nullable(String),
    `dispute_id` Nullable(String),
    `payout_id` Nullable(String),
    `destination` LowCardinality(Nullable(String)),
    `execution_mode` LowCardinality(Nullable(String)),
    `source` LowCardinality(Nullable(String))
) AS
SELECT
    merchant_id,
    payment_id,
    connector_name,
    request_id,
    flow,
    request,
    masked_response AS response,
    masked_response,
    error,
    status_code,
    created_at,
    now64() AS inserted_at,
    latency,
    method,
    refund_id,
    dispute_id,
    payout_id,
    destination,
    execution_mode,
    'hyperswitch' AS source
FROM
    connector_events_queue
WHERE
    length(_error) = 0;

-- Kafka consumer for the UCS connector-events topic; maps UCS's native field
-- shape onto the shared connector_events tables (source = unified_connector_service).

CREATE TABLE unified_connector_service_connector_events_queue
(
    `request_id` String,
    `timestamp` Int64,
    `flow_type` LowCardinality(String),
    `connector` LowCardinality(String),
    `url` Nullable(String),
    `method` Nullable(String),
    `stage` LowCardinality(String),
    `execution_mode` LowCardinality(Nullable(String)),
    `latency_ms` Nullable(UInt64),
    `status_code` Nullable(Int32),
    `request_data` Nullable(String),
    `response_data` Nullable(String),
    `error` Nullable(String),
    `reference_id` Nullable(String),
    `resource_id` Nullable(String),
    `lineage_merchant_id` Nullable(String)
)
ENGINE = Kafka
SETTINGS kafka_broker_list = 'kafka0:29092', kafka_topic_list = 'unified-connector-service-connector-events', kafka_group_name = 'hyper', kafka_format = 'JSONEachRow', kafka_handle_error_mode = 'stream';

CREATE MATERIALIZED VIEW unified_connector_service_connector_events_parse_errors (
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
    unified_connector_service_connector_events_queue
WHERE
    length(_error) > 0;

CREATE MATERIALIZED VIEW unified_connector_service_connector_events_mv TO connector_events (
    `merchant_id` String,
    `payment_id` Nullable(String),
    `connector_name` LowCardinality(String),
    `request_id` String,
    `flow` LowCardinality(String),
    `request` String,
    `response` Nullable(String),
    `masked_response` Nullable(String),
    `error` Nullable(String),
    `status_code` UInt32,
    `created_at` DateTime64(3),
    `inserted_at` DateTime DEFAULT now() CODEC(T64, LZ4),
    `latency` UInt128,
    `method` LowCardinality(String),
    `refund_id` Nullable(String),
    `payout_id` Nullable(String),
    `destination` LowCardinality(Nullable(String)),
    `execution_mode` LowCardinality(Nullable(String)),
    `source` LowCardinality(Nullable(String))
) AS
SELECT
    ifNull(lineage_merchant_id, '') AS merchant_id,
    reference_id AS payment_id,
    connector AS connector_name,
    request_id,
    flow_type AS flow,
    ifNull(request_data, '') AS request,
    response_data AS response,
    response_data AS masked_response,
    error,
    toUInt32(ifNull(status_code, 0)) AS status_code,
    fromUnixTimestamp64Milli(timestamp) AS created_at,
    now64() AS inserted_at,
    toUInt128(ifNull(latency_ms, 0)) AS latency,
    ifNull(method, '') AS method,
    resource_id AS refund_id,
    CAST(NULL AS Nullable(String)) AS payout_id,
    'connector' AS destination,
    execution_mode,
    'unified_connector_service' AS source
FROM
    unified_connector_service_connector_events_queue
WHERE
    length(_error) = 0;

CREATE MATERIALIZED VIEW unified_connector_service_connector_events_audit_mv TO connector_events_audit (
    `merchant_id` String,
    `payment_id` String,
    `connector_name` LowCardinality(String),
    `request_id` String,
    `flow` LowCardinality(String),
    `request` String,
    `response` Nullable(String),
    `masked_response` Nullable(String),
    `error` Nullable(String),
    `status_code` UInt32,
    `created_at` DateTime64(3),
    `inserted_at` DateTime DEFAULT now() CODEC(T64, LZ4),
    `latency` UInt128,
    `method` LowCardinality(String),
    `refund_id` Nullable(String),
    `destination` LowCardinality(Nullable(String)),
    `execution_mode` LowCardinality(Nullable(String)),
    `source` LowCardinality(Nullable(String))
) AS
SELECT
    ifNull(lineage_merchant_id, '') AS merchant_id,
    reference_id AS payment_id,
    connector AS connector_name,
    request_id,
    flow_type AS flow,
    ifNull(request_data, '') AS request,
    response_data AS response,
    response_data AS masked_response,
    error,
    toUInt32(ifNull(status_code, 0)) AS status_code,
    fromUnixTimestamp64Milli(timestamp) AS created_at,
    now64() AS inserted_at,
    toUInt128(ifNull(latency_ms, 0)) AS latency,
    ifNull(method, '') AS method,
    resource_id AS refund_id,
    'connector' AS destination,
    execution_mode,
    'unified_connector_service' AS source
FROM
    unified_connector_service_connector_events_queue
WHERE
    (length(_error) = 0)
    AND (reference_id IS NOT NULL);
