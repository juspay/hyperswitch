CREATE TABLE dispute_queue (
    `dispute_id` String,
    `amount` String,
    `currency` String,
    `dispute_stage` LowCardinality(String),
    `dispute_status` LowCardinality(String),
    `payment_id` String,
    `attempt_id` String,
    `merchant_id` String,
    `connector_status` String,
    `connector_dispute_id` String,
    `connector_reason` Nullable(String),
    `connector_reason_code` Nullable(String),
    `challenge_required_by` Nullable(DateTime) CODEC(T64, LZ4),
    `connector_created_at` Nullable(DateTime) CODEC(T64, LZ4),
    `connector_updated_at` Nullable(DateTime) CODEC(T64, LZ4),
    `created_at` DateTime CODEC(T64, LZ4),
    `modified_at` DateTime CODEC(T64, LZ4),
    `connector` LowCardinality(String),
    `evidence` Nullable(String),
    `profile_id` Nullable(String),
    `merchant_connector_id` Nullable(String),
    `sign_flag` Int8
) ENGINE = Kafka SETTINGS kafka_broker_list = 'kafka0:29092',
kafka_topic_list = 'hyperswitch-dispute-events',
kafka_group_name = 'hyper-c1',
kafka_format = 'JSONEachRow',
kafka_handle_error_mode = 'stream';


CREATE TABLE dispute (
    `dispute_id` String,
    `amount` String,
    `currency` String,
    `dispute_stage` LowCardinality(String),
    `dispute_status` LowCardinality(String),
    `payment_id` String,
    `attempt_id` String,
    `merchant_id` String,
    `connector_status` String,
    `connector_dispute_id` String,
    `connector_reason` Nullable(String),
    `connector_reason_code` Nullable(String),
    `challenge_required_by` Nullable(DateTime) CODEC(T64, LZ4),
    `connector_created_at` Nullable(DateTime) CODEC(T64, LZ4),
    `connector_updated_at` Nullable(DateTime) CODEC(T64, LZ4),
    `created_at` DateTime DEFAULT now() CODEC(T64, LZ4),
    `modified_at` DateTime DEFAULT now() CODEC(T64, LZ4),
    `connector` LowCardinality(String),
    `evidence` String DEFAULT '{}' CODEC(T64, LZ4),
    `profile_id` Nullable(String),
    `merchant_connector_id` Nullable(String),
    `inserted_at` DateTime DEFAULT now() CODEC(T64, LZ4),
    `sign_flag` Int8
    INDEX connectorIndex connector TYPE bloom_filter GRANULARITY 1,
    INDEX disputeStatusIndex dispute_status TYPE bloom_filter GRANULARITY 1,
    INDEX disputeStageIndex dispute_stage TYPE bloom_filter GRANULARITY 1
) ENGINE = CollapsingMergeTree(
    sign_flag
)
PARTITION BY toStartOfDay(created_at)
ORDER BY
    (created_at, merchant_id, dispute_id)
TTL created_at + toIntervalMonth(6)
;

CREATE MATERIALIZED VIEW kafka_parse_dispute TO dispute (
    `dispute_id` String,
    `amount` String,
    `currency` String,
    `dispute_stage` LowCardinality(String),
    `dispute_status` LowCardinality(String),
    `payment_id` String,
    `attempt_id` String,
    `merchant_id` String,
    `connector_status` String,
    `connector_dispute_id` String,
    `connector_reason` Nullable(String),
    `connector_reason_code` Nullable(String),
    `challenge_required_by` Nullable(DateTime64(3)),
    `connector_created_at` Nullable(DateTime64(3)),
    `connector_updated_at` Nullable(DateTime64(3)),
    `created_at` DateTime64(3),
    `modified_at` DateTime64(3),
    `connector` LowCardinality(String),
    `evidence` Nullable(String),
    `profile_id` Nullable(String),
    `merchant_connector_id` Nullable(String),
    `inserted_at` DateTime64(3),
    `sign_flag` Int8
) AS
SELECT
    dispute_id,
    amount,
    currency,
    dispute_stage,
    dispute_status,
    payment_id,
    attempt_id,
    merchant_id,
    connector_status,
    connector_dispute_id,
    connector_reason,
    connector_reason_code,
    challenge_required_by,
    connector_created_at,
    connector_updated_at,
    created_at,
    modified_at,
    connector,
    evidence,
    profile_id,
    merchant_connector_id,
    now() as inserted_at,
    sign_flag
FROM
    dispute_queue;
