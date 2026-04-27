-- =====================================================================
-- ClickHouse Observability Schema - Correlated Events Pipeline
-- =====================================================================
-- This schema is designed for the multi-event correlation architecture
-- where span events are correlated before being published to Kafka.
--
-- Version: 1.0
-- Date: 2026-03-18
-- Target ClickHouse Version: 24.x+
--
-- Architecture:
--   Application (Span + Request Events) -> Correlation Service -> Kafka
--   -> api_observability_events_queue (Kafka Engine)
--      -> MV -> api_observability_events_flat (Primary, one row per span)
--      -> MV -> api_observability_events (Secondary, one row per request with Nested)
--
-- The correlation service outputs array-shaped wide events identical
-- to the current application middleware output. No changes to ClickHouse
-- schema from the proven configuration.
-- =====================================================================

-- ----------------------------------------------------------------------
-- 1. KAFKA QUEUE TABLE
-- ----------------------------------------------------------------------
-- Entry point for all data. Reads from Kafka. Stores nothing on disk.
-- kafka_handle_error_mode='stream' sends malformed messages to _error
-- virtual column instead of failing the entire batch.
-- ----------------------------------------------------------------------
CREATE TABLE api_observability_events_queue
(
    tenant_id              String,
    merchant_id            String,
    api_flow               String,
    request_id             String,
    flow_type              String,
    status_code            UInt32,
    api_auth_type          String,
    request                Nullable(String),
    response               Nullable(String),
    error                  Nullable(String),
    user_agent             String,
    ip_addr                String,
    url_path               Nullable(String),
    http_method            String,
    latency                UInt64,
    hs_latency             Nullable(UInt64),
    created_at             DateTime64(3),
    external_service_calls Array(Tuple(
        service_name  String,
        endpoint      String,
        method        String,
        status_code   UInt16,
        success       Bool,
        latency_ms    UInt32,
        metadata      String
    ))
)
ENGINE = Kafka
SETTINGS
    kafka_broker_list       = 'kafka:9092',
    kafka_topic_list        = 'hyperswitch-api-observability-events',
    kafka_group_name        = 'clickhouse-consumer',
    kafka_format            = 'JSONEachRow',
    kafka_handle_error_mode = 'stream';

-- ----------------------------------------------------------------------
-- 2. PRIMARY STORAGE TABLE: api_observability_events_flat
-- ----------------------------------------------------------------------
-- One row per external service call. Serves ~90% of queries.
-- Optimized for low-cardinality GROUP BY aggregations.
--
-- ORDER BY: (service_name, merchant_id, created_at, request_status_code)
--   - service_name first: most common filter is service_name = 'KeyManager'
--   - merchant_id second: second most common filter dimension
--   - created_at third: enables efficient time-range scans
--   - request_status_code last: useful for error-rate queries
--
-- KEY DESIGN CHOICES:
--   - request_latency_ms denormalized on every span row (no join needed)
--   - endpoint is String not LowCardinality (cardinality may exceed 10K)
--   - Monthly partitions (daily would give ~530K rows - too small)
--   - Large fields use ZSTD(1) and are placed last
--   - request/response/error excluded (live in api_observability_events only)
-- ----------------------------------------------------------------------
CREATE TABLE api_observability_events_flat
(
    request_id              String,
    tenant_id               LowCardinality(String)   DEFAULT '',
    merchant_id             LowCardinality(String)   DEFAULT '',
    api_flow                LowCardinality(String)   DEFAULT '',
    flow_type               LowCardinality(String)   DEFAULT '',
    api_auth_type           LowCardinality(String)   DEFAULT '',
    http_method             LowCardinality(String)   DEFAULT '',
    request_status_code     UInt16                   DEFAULT 0,
    request_latency_ms      UInt32                   DEFAULT 0,
    service_name            LowCardinality(String)   DEFAULT '',
    endpoint                String                   DEFAULT '' CODEC(ZSTD(1)),
    ext_status_code         UInt16                   DEFAULT 0,
    success                 Bool                     DEFAULT true,
    call_latency_ms         UInt32                   DEFAULT 0,
    created_at              DateTime64(3),
    inserted_at             DateTime                 DEFAULT now() CODEC(T64, LZ4),
    url_path                String                   DEFAULT '' CODEC(ZSTD(1)),
    ip_addr                 String                   DEFAULT '' CODEC(ZSTD(1)),
    user_agent              String                   DEFAULT '' CODEC(ZSTD(1)),
    INDEX idx_service       service_name             TYPE bloom_filter GRANULARITY 1,
    INDEX idx_flow          api_flow                 TYPE bloom_filter GRANULARITY 1,
    INDEX idx_success       success                  TYPE bloom_filter GRANULARITY 1,
    INDEX idx_ext_status    ext_status_code          TYPE bloom_filter GRANULARITY 1
)
ENGINE = MergeTree()
PARTITION BY toYYYYMM(created_at)
ORDER BY (service_name, merchant_id, created_at, request_status_code)
TTL inserted_at + toIntervalMonth(18)
SETTINGS index_granularity = 8192;

-- ----------------------------------------------------------------------
-- 3. SECONDARY STORAGE TABLE: api_observability_events
-- ----------------------------------------------------------------------
-- One row per API request with all external service calls in Nested column.
-- Used exclusively for per-request correlation queries (arrayCount, arrayExists).
--
-- WHY Nested instead of Array(Tuple):
--   Nested stores each sub-column as a separate Array(T) column on disk.
--   ClickHouse reads only the sub-columns referenced in the query.
--   Array(Tuple) stores the full tuple blob together.
--
-- ORDER BY: (merchant_id, created_at)
--   Queries on this table are scoped to request_id or merchant's requests,
--   not specific service.
--
-- Includes request/response/error blobs (not in api_observability_events_flat).
-- ----------------------------------------------------------------------
CREATE TABLE api_observability_events
(
    request_id              String,
    tenant_id               LowCardinality(String)   DEFAULT '',
    merchant_id             LowCardinality(String)   DEFAULT '',
    api_flow                LowCardinality(String)   DEFAULT '',
    flow_type               LowCardinality(String)   DEFAULT '',
    api_auth_type           LowCardinality(String)   DEFAULT '',
    http_method             LowCardinality(String)   DEFAULT '',
    status_code             UInt16                   DEFAULT 0,
    latency_ms              UInt32                   DEFAULT 0,
    url_path                String                   DEFAULT '',
    ip_addr                 String                   DEFAULT '',
    user_agent              String                   DEFAULT '',
    request                 Nullable(String),
    response                Nullable(String),
    error                   Nullable(String),
    external_service_calls  Nested(
        service_name        String,
        endpoint            String,
        method              String,
        status_code         UInt16,
        success             Bool,
        latency_ms          UInt32,
        metadata            String
    ),
    created_at              DateTime64(3),
    inserted_at             DateTime                 DEFAULT now() CODEC(T64, LZ4)
)
ENGINE = MergeTree()
PARTITION BY toYYYYMM(created_at)
ORDER BY (merchant_id, created_at)
TTL inserted_at + toIntervalMonth(18)
SETTINGS index_granularity = 8192;

-- ----------------------------------------------------------------------
-- 4. MATERIALIZED VIEW: api_observability_events_flat_mv
-- ----------------------------------------------------------------------
-- Transforms: array-shaped Kafka message -> one flat row per span
-- HOW: ARRAY JOIN explodes external_service_calls array at insert time.
--      One-time compute cost. All subsequent reads are free.
-- NOTE: ext.1..ext.6 are positional tuple accessors. Tuple order in
--       api_observability_events_queue: (service_name, endpoint, method, status_code, success, latency_ms, metadata)
-- ----------------------------------------------------------------------
CREATE MATERIALIZED VIEW api_observability_events_flat_mv
TO api_observability_events_flat AS
SELECT
    request_id,
    tenant_id,
    merchant_id,
    api_flow,
    flow_type,
    api_auth_type,
    http_method,
    toUInt16(status_code)           AS request_status_code,
    toUInt32(latency / 1000)        AS request_latency_ms,
    ext.1                           AS service_name,
    ext.2                           AS endpoint,
    toUInt16(ext.4)                 AS ext_status_code,
    ext.5                           AS success,
    toUInt32(ext.6)                 AS call_latency_ms,
    created_at,
    now()                           AS inserted_at,
    coalesce(url_path, '')          AS url_path,
    ip_addr,
    user_agent
FROM api_observability_events_queue
ARRAY JOIN external_service_calls AS ext
WHERE length(_error) = 0;

-- ----------------------------------------------------------------------
-- 5. MATERIALIZED VIEW: api_observability_events_mv
-- ----------------------------------------------------------------------
-- Transforms: array-shaped Kafka message -> one row per request with Nested
-- HOW: No ARRAY JOIN. Array is preserved as-is.
--      Each sub-column extracted from tuple array via arrayMap.
-- ----------------------------------------------------------------------
CREATE MATERIALIZED VIEW api_observability_events_mv
TO api_observability_events AS
SELECT
    request_id,
    tenant_id,
    merchant_id,
    api_flow,
    flow_type,
    api_auth_type,
    http_method,
    toUInt16(status_code)                    AS status_code,
    toUInt32(latency / 1000)                 AS latency_ms,
    coalesce(url_path, '')                   AS url_path,
    ip_addr,
    user_agent,
    request,
    response,
    error,
    arrayMap(x -> x.1, external_service_calls) AS `external_service_calls.service_name`,
    arrayMap(x -> x.2, external_service_calls) AS `external_service_calls.endpoint`,
    arrayMap(x -> x.3, external_service_calls) AS `external_service_calls.method`,
    arrayMap(x -> x.4, external_service_calls) AS `external_service_calls.status_code`,
    arrayMap(x -> x.5, external_service_calls) AS `external_service_calls.success`,
    arrayMap(x -> x.6, external_service_calls) AS `external_service_calls.latency_ms`,
    arrayMap(x -> x.7, external_service_calls) AS `external_service_calls.metadata`,
    created_at,
    now()                                    AS inserted_at
FROM api_observability_events_queue
WHERE length(_error) = 0;