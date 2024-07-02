CREATE TABLE authentication_queue (
    `authentication_id` String,
    `merchant_id` String,
    `authentication_connector` LowCardinality(String),
    `connector_authentication_id` Nullable(String),
    `authentication_data` Nullable(String),
    `payment_method_id` Nullable(String),
    `authentication_type` LowCardinality(Nullable(String)),
    `authentication_status` LowCardinality(String),
    `authentication_lifecycle_status` LowCardinality(String),
    `created_at` DateTime64(3),
    `modified_at` DateTime64(3),
    `error_message` Nullable(String),
    `error_code` Nullable(String),
    `connector_metadata` Nullable(String),
    `maximum_supported_version` LowCardinality(Nullable(String)),
    `threeds_server_transaction_id` Nullable(String),
    `cavv` Nullable(String),
    `authentication_flow_type` Nullable(String),
    `message_version` LowCardinality(Nullable(String)),
    `eci` Nullable(String),
    `trans_status` LowCardinality(Nullable(String)),
    `acquirer_bin` Nullable(String),
    `acquirer_merchant_id` Nullable(String),
    `three_ds_method_data` Nullable(String),
    `three_ds_method_url` Nullable(String),
    `acs_url` Nullable(String),
    `challenge_request` Nullable(String),
    `acs_reference_number` Nullable(String),
    `acs_trans_id` Nullable(String),
    `acs_signed_content` Nullable(String),
    `profile_id` String,
    `payment_id` Nullable(String),
    `merchant_connector_id` Nullable(String),
    `ds_trans_id` Nullable(String),
    `directory_server_id` Nullable(String),
    `acquirer_country_code` Nullable(String),
    `sign_flag` Int8
) ENGINE = Kafka SETTINGS kafka_broker_list = 'kafka0:29092',
kafka_topic_list = 'hyperswitch-authentication-events',
kafka_group_name = 'hyper',
kafka_format = 'JSONEachRow',
kafka_handle_error_mode = 'stream';

CREATE TABLE authentications (
    `authentication_id` String,
    `merchant_id` String,
    `authentication_connector` LowCardinality(String),
    `connector_authentication_id` Nullable(String),
    `authentication_data` Nullable(String),
    `payment_method_id` Nullable(String),
    `authentication_type` LowCardinality(Nullable(String)),
    `authentication_status` LowCardinality(String),
    `authentication_lifecycle_status` LowCardinality(String),
    `created_at` DateTime64(3) DEFAULT now64(),
    `inserted_at` DateTime64(3) DEFAULT now64(),
    `modified_at` DateTime64(3) DEFAULT now64(),
    `error_message` Nullable(String),
    `error_code` Nullable(String),
    `connector_metadata` Nullable(String),
    `maximum_supported_version` LowCardinality(Nullable(String)),
    `threeds_server_transaction_id` Nullable(String),
    `cavv` Nullable(String),
    `authentication_flow_type` Nullable(String),
    `message_version` LowCardinality(Nullable(String)),
    `eci` Nullable(String),
    `trans_status` LowCardinality(Nullable(String)),
    `acquirer_bin` Nullable(String),
    `acquirer_merchant_id` Nullable(String),
    `three_ds_method_data` Nullable(String),
    `three_ds_method_url` Nullable(String),
    `acs_url` Nullable(String),
    `challenge_request` Nullable(String),
    `acs_reference_number` Nullable(String),
    `acs_trans_id` Nullable(String),
    `acs_signed_content` Nullable(String),
    `profile_id` String,
    `payment_id` Nullable(String),
    `merchant_connector_id` Nullable(String),
    `ds_trans_id` Nullable(String),
    `directory_server_id` Nullable(String),
    `acquirer_country_code` Nullable(String),
    `sign_flag` Int8,
    INDEX authenticationConnectorIndex authentication_connector TYPE bloom_filter GRANULARITY 1,
    INDEX transStatusIndex trans_status TYPE bloom_filter GRANULARITY 1,
    INDEX authenticationTypeIndex authentication_type TYPE bloom_filter GRANULARITY 1,
    INDEX authenticationStatusIndex authentication_status TYPE bloom_filter GRANULARITY 1
) ENGINE = CollapsingMergeTree(sign_flag) PARTITION BY toStartOfDay(created_at)
ORDER BY
    (created_at, merchant_id, authentication_id) TTL toStartOfDay(created_at) + toIntervalMonth(18) SETTINGS index_granularity = 8192;

CREATE MATERIALIZED VIEW authentication_mv TO authentications (
    `authentication_id` String,
    `merchant_id` String,
    `authentication_connector` LowCardinality(String),
    `connector_authentication_id` Nullable(String),
    `authentication_data` Nullable(String),
    `payment_method_id` Nullable(String),
    `authentication_type` LowCardinality(Nullable(String)),
    `authentication_status` LowCardinality(String),
    `authentication_lifecycle_status` LowCardinality(String),
    `created_at` DateTime64(3) DEFAULT now64(),
    `inserted_at` DateTime64(3) DEFAULT now64(),
    `modified_at` DateTime64(3) DEFAULT now64(),
    `error_message` Nullable(String),
    `error_code` Nullable(String),
    `connector_metadata` Nullable(String),
    `maximum_supported_version` LowCardinality(Nullable(String)),
    `threeds_server_transaction_id` Nullable(String),
    `cavv` Nullable(String),
    `authentication_flow_type` Nullable(String),
    `message_version` LowCardinality(Nullable(String)),
    `eci` Nullable(String),
    `trans_status` LowCardinality(Nullable(String)),
    `acquirer_bin` Nullable(String),
    `acquirer_merchant_id` Nullable(String),
    `three_ds_method_data` Nullable(String),
    `three_ds_method_url` Nullable(String),
    `acs_url` Nullable(String),
    `challenge_request` Nullable(String),
    `acs_reference_number` Nullable(String),
    `acs_trans_id` Nullable(String),
    `acs_signed_content` Nullable(String),
    `profile_id` String,
    `payment_id` Nullable(String),
    `merchant_connector_id` Nullable(String),
    `ds_trans_id` Nullable(String),
    `directory_server_id` Nullable(String),
    `acquirer_country_code` Nullable(String),
    `sign_flag` Int8
) AS
SELECT
    authentication_id,
    merchant_id,
    authentication_connector,
    connector_authentication_id,
    authentication_data,
    payment_method_id,
    authentication_type,
    authentication_status,
    authentication_lifecycle_status,
    created_at,
    now64() as inserted_at,
    modified_at,
    error_message,
    error_code,
    connector_metadata,
    maximum_supported_version,
    threeds_server_transaction_id,
    cavv,
    authentication_flow_type,
    message_version,
    eci,
    trans_status,
    acquirer_bin,
    acquirer_merchant_id,
    three_ds_method_data,
    three_ds_method_url,
    acs_url,
    challenge_request,
    acs_reference_number,
    acs_trans_id,
    acs_signed_content,
    profile_id,
    payment_id,
    merchant_connector_id,
    ds_trans_id,
    directory_server_id,
    acquirer_country_code,
    sign_flag
FROM
    authentication_queue
WHERE
    length(_error) = 0;

CREATE MATERIALIZED VIEW authentication_parse_errors (
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
    authentication_queue
WHERE
    length(_error) > 0;