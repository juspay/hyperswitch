CREATE TABLE hyperswitch.payment_intents_queue on cluster '{cluster}' (
    `payment_id` String,
    `merchant_id` String,
    `status` LowCardinality(String),
    `amount` UInt32,
    `currency` LowCardinality(Nullable(String)),
    `amount_captured` Nullable(UInt32),
    `customer_id` Nullable(String),
    `description` Nullable(String),
    `return_url` Nullable(String),
    `connector_id` LowCardinality(Nullable(String)),
    `statement_descriptor_name` Nullable(String),
    `statement_descriptor_suffix` Nullable(String),
    `setup_future_usage` LowCardinality(Nullable(String)),
    `off_session` Nullable(Bool),
    `client_secret` Nullable(String),
    `active_attempt_id` String,
    `business_country` String,
    `business_label` String,
    `modified_at` DateTime,
    `created_at` DateTime,
    `last_synced` Nullable(DateTime) CODEC(T64, LZ4),
    `sign_flag` Int8
) ENGINE = Kafka SETTINGS kafka_broker_list = 'kafka0:29092',
kafka_topic_list = 'hyperswitch-payment-intent-events',
kafka_group_name = 'hyper-c1',
kafka_format = 'JSONEachRow',
kafka_handle_error_mode = 'stream';

CREATE TABLE hyperswitch.payment_intents_dist on cluster '{cluster}' (
    `payment_id` String,
    `merchant_id` String,
    `status` LowCardinality(String),
    `amount` UInt32,
    `currency` LowCardinality(Nullable(String)),
    `amount_captured` Nullable(UInt32),
    `customer_id` Nullable(String),
    `description` Nullable(String),
    `return_url` Nullable(String),
    `connector_id` LowCardinality(Nullable(String)),
    `statement_descriptor_name` Nullable(String),
    `statement_descriptor_suffix` Nullable(String),
    `setup_future_usage` LowCardinality(Nullable(String)),
    `off_session` Nullable(Bool),
    `client_secret` Nullable(String),
    `active_attempt_id` String,
    `business_country` LowCardinality(String),
    `business_label` String,
    `modified_at` DateTime DEFAULT now() CODEC(T64, LZ4),
    `created_at` DateTime DEFAULT now() CODEC(T64, LZ4),
    `last_synced` Nullable(DateTime) CODEC(T64, LZ4),
    `inserted_at` DateTime DEFAULT now() CODEC(T64, LZ4),
    `sign_flag` Int8
) ENGINE = Distributed('{cluster}', 'hyperswitch', 'payment_intents_clustered', cityHash64(payment_id));

CREATE TABLE hyperswitch.payment_intents_clustered on cluster '{cluster}' (
    `payment_id` String,
    `merchant_id` String,
    `status` LowCardinality(String),
    `amount` UInt32,
    `currency` LowCardinality(Nullable(String)),
    `amount_captured` Nullable(UInt32),
    `customer_id` Nullable(String),
    `description` Nullable(String),
    `return_url` Nullable(String),
    `connector_id` LowCardinality(Nullable(String)),
    `statement_descriptor_name` Nullable(String),
    `statement_descriptor_suffix` Nullable(String),
    `setup_future_usage` LowCardinality(Nullable(String)),
    `off_session` Nullable(Bool),
    `client_secret` Nullable(String),
    `active_attempt_id` String,
    `business_country` LowCardinality(String),
    `business_label` String,
    `modified_at` DateTime DEFAULT now() CODEC(T64, LZ4),
    `created_at` DateTime DEFAULT now() CODEC(T64, LZ4),
    `last_synced` Nullable(DateTime) CODEC(T64, LZ4),
    `inserted_at` DateTime DEFAULT now() CODEC(T64, LZ4),
    `sign_flag` Int8,
    INDEX connectorIndex connector_id TYPE bloom_filter GRANULARITY 1,
    INDEX currencyIndex currency TYPE bloom_filter GRANULARITY 1,
    INDEX statusIndex status TYPE bloom_filter GRANULARITY 1
) ENGINE = ReplicatedCollapsingMergeTree(
    '/clickhouse/{installation}/{cluster}/tables/{shard}/hyperswitch/payment_intents_clustered',
    '{replica}',
    sign_flag
)
PARTITION BY toStartOfDay(created_at)
ORDER BY
    (created_at, merchant_id, payment_id)
TTL created_at + toIntervalMonth(6)
;

CREATE MATERIALIZED VIEW hyperswitch.payment_intent_mv on cluster '{cluster}' TO hyperswitch.payment_intents_dist (
    `payment_id` String,
    `merchant_id` String,
    `status` LowCardinality(String),
    `amount` UInt32,
    `currency` LowCardinality(Nullable(String)),
    `amount_captured` Nullable(UInt32),
    `customer_id` Nullable(String),
    `description` Nullable(String),
    `return_url` Nullable(String),
    `connector_id` LowCardinality(Nullable(String)),
    `statement_descriptor_name` Nullable(String),
    `statement_descriptor_suffix` Nullable(String),
    `setup_future_usage` LowCardinality(Nullable(String)),
    `off_session` Nullable(Bool),
    `client_secret` Nullable(String),
    `active_attempt_id` String,
    `business_country` LowCardinality(String),
    `business_label` String,
    `modified_at` DateTime64(3),
    `created_at` DateTime64(3),
    `last_synced` Nullable(DateTime64(3)),
    `inserted_at` DateTime64(3),
    `sign_flag` Int8
) AS
SELECT
    payment_id,
    merchant_id,
    status,
    amount,
    currency,
    amount_captured,
    customer_id,
    description,
    return_url,
    connector_id,
    statement_descriptor_name,
    statement_descriptor_suffix,
    setup_future_usage,
    off_session,
    client_secret,
    active_attempt_id,
    business_country,
    business_label,
    modified_at,
    created_at,
    last_synced,
    now() as inserted_at,
    sign_flag
FROM hyperswitch.payment_intents_queue
WHERE length(_error) = 0;

CREATE MATERIALIZED VIEW hyperswitch.payment_intent_parse_errors on cluster '{cluster}'
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
FROM hyperswitch.payment_intents_queue
WHERE length(_error) > 0
;
