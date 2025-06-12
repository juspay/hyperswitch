CREATE TABLE merchant_account_queue (
    `merchant_id` String,
    `publishable_key` String,
    `sign_flag` Int8
) ENGINE = Kafka SETTINGS kafka_broker_list = 'kafka0:29092',
kafka_topic_list = 'hyperswitch-merchant-account',
kafka_group_name = 'hyper',
kafka_format = 'JSONEachRow',
kafka_handle_error_mode = 'stream';

CREATE TABLE  merchant_account(
    `merchant_id` String,
    `publishable_key` String,
    `sign_flag` Int8,
    INDEX publishableKeyIndex publishable_key TYPE bloom_filter GRANULARITY 1,
) ENGINE = CollapsingMergeTree(sign_flag) PARTITION BY toStartOfDay(created_at)
ORDER BY
    (publishable_key) TTL toStartOfDay(created_at) + toIntervalMonth(18) SETTINGS index_granularity = 8192;

CREATE MATERIALIZED VIEW merchant_account_mv TO merchant_account (
    `merchant_id` String,
    `publishable_key` String,
    `sign_flag` Int8,
) AS
SELECT
    merchant_id,
    publishable_key,
    sign_flag
FROM
    merchant_account_queue
WHERE
    length(_error) = 0;

CREATE MATERIALIZED VIEW  merchant_account_parse_errors (
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
    merchant_account_queue
WHERE
    length(_error) > 0;