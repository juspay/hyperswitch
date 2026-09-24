-- Prism (Unified Connector Service) connector events.
--
-- Normalized, payment/payout-scoped connector events read by the analytics API
-- (ConnectorEventSource::Prism -> prism_connector_events_audit /
-- prism_connector_events_payout_audit).
--
-- Adjust kafka_broker_list / kafka_topic_list / kafka_group_name to match your
-- deployment before running these scripts.

CREATE TABLE prism_connector_events_queue
(
    `tenant_id` String,
    `merchant_id` String,
    `payment_id` Nullable(String),
    `connector_name` LowCardinality(String),
    `request_id` String,
    `url` Nullable(String),
    `flow` LowCardinality(String),
    `request` String,
    `masked_response` Nullable(String),
    `error` Nullable(String),
    `status_code` UInt32,
    `created_at` DateTime64(3),
    `latency` UInt128,
    `method` LowCardinality(String),
    `refund_id` Nullable(String),
    `dispute_id` Nullable(String),
    `payout_id` Nullable(String),
    `service_name` LowCardinality(Nullable(String)),
    `execution_mode` LowCardinality(Nullable(String)),
    `destination` LowCardinality(Nullable(String))
)
ENGINE = Kafka
SETTINGS kafka_broker_list = 'kafka0:29092', kafka_topic_list = 'prism-outgoing-connector-events', kafka_group_name = 'hyper', kafka_format = 'JSONEachRow', kafka_handle_error_mode = 'stream';

CREATE MATERIALIZED VIEW prism_connector_events_parse_errors (
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
    prism_connector_events_queue
WHERE
    length(_error) > 0;

CREATE TABLE prism_connector_events_audit (
    `merchant_id` String,
    `payment_id` String,
    `connector_name` LowCardinality(String),
    `request_id` String,
    `url` Nullable(String),
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
    INDEX flowIndex flow TYPE bloom_filter GRANULARITY 1,
    INDEX connectorIndex connector_name TYPE bloom_filter GRANULARITY 1,
    INDEX statusIndex status_code TYPE bloom_filter GRANULARITY 1
) ENGINE = MergeTree PARTITION BY merchant_id
ORDER BY
    (merchant_id, payment_id) SETTINGS index_granularity = 8192;

CREATE TABLE prism_connector_events_payout_audit (
    `merchant_id` String,
    `payout_id` String,
    `connector_name` LowCardinality(String),
    `request_id` String,
    `url` Nullable(String),
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
    INDEX flowIndex flow TYPE bloom_filter GRANULARITY 1,
    INDEX connectorIndex connector_name TYPE bloom_filter GRANULARITY 1,
    INDEX statusIndex status_code TYPE bloom_filter GRANULARITY 1
) ENGINE = MergeTree PARTITION BY merchant_id
ORDER BY
    (merchant_id, payout_id) SETTINGS index_granularity = 8192;

CREATE MATERIALIZED VIEW prism_connector_events_audit_mv TO prism_connector_events_audit (
    `merchant_id` String,
    `payment_id` Nullable(String),
    `connector_name` LowCardinality(String),
    `request_id` String,
    `url` Nullable(String),
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
    `execution_mode` LowCardinality(Nullable(String))
) AS
SELECT
    merchant_id,
    assumeNotNull(payment_id) AS payment_id,
    connector_name,
    request_id,
    url,
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
    execution_mode
FROM
    prism_connector_events_queue
WHERE
    (length(_error) = 0)
    AND (tenant_id = 'public')
    AND notEmpty(ifNull(payment_id, ''));

CREATE MATERIALIZED VIEW prism_connector_events_payout_audit_mv TO prism_connector_events_payout_audit (
    `merchant_id` String,
    `payout_id` Nullable(String),
    `connector_name` LowCardinality(String),
    `request_id` String,
    `url` Nullable(String),
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
    `execution_mode` LowCardinality(Nullable(String))
) AS
SELECT
    merchant_id,
    assumeNotNull(payout_id) AS payout_id,
    connector_name,
    request_id,
    url,
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
    execution_mode
FROM
    prism_connector_events_queue
WHERE
    (length(_error) = 0)
    AND (tenant_id = 'public')
    AND notEmpty(ifNull(payout_id, ''));
