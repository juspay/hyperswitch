-- The observability plane's own state.
--
-- These tables are not part of hyperswitch_db. They live in a database this plane owns, so that
-- alert state is not written into a store the application owns.
--
-- Two things are worth knowing before changing a column here:
--
--   * `json` and `jsonb` are not interchangeable. The columns holding values the dashboard sends
--     already serialised are `json`, which preserves the bytes it was given; `jsonb` re-encodes
--     and would hand back something different from what was saved.
--   * `alerts_main` records each announcement that was sent, separately from `alerts_intermediate`,
--     which holds current lifecycle state. Keeping them apart is what makes it possible to say
--     whether an alert was delivered.
--
-- Timestamps are naive TIMESTAMP, as elsewhere in this repository, which holds UTC by discipline
-- rather than by column type.

-- Alert definitions: one row per alert, and the whole of its configuration.
-- blacklist, snooze and thresholds live here as JSON rather than as side tables,
-- so everything about an alert is in one place.
CREATE TABLE IF NOT EXISTS alerts_info (
    id               UUID DEFAULT gen_random_uuid() PRIMARY KEY,
    name             VARCHAR(64) NOT NULL,
    product          VARCHAR(64) NOT NULL,
    dimensions       VARCHAR(255),
    period           INTEGER,
    default_channel  VARCHAR(64),
    default_critical BOOLEAN,
    blacklist        JSON,
    snooze           JSON,
    history_window   INTEGER,
    thresholds       JSON,
    metadata         JSON,
    is_enabled       BOOLEAN DEFAULT FALSE,
    comments         JSON,
    call_period      INTEGER,
    author           VARCHAR(64) DEFAULT 'reliability_team',
    approver         VARCHAR(64),
    last_updated_at  TIMESTAMP DEFAULT date_trunc('second', CURRENT_TIMESTAMP)
);

-- Announcements actually sent, one row per alert per delivery. `sent` and
-- `ts_slack` are the record that an alert reached a channel; without them a
-- failed delivery is indistinguishable from a successful one on the next run.
CREATE TABLE IF NOT EXISTS alerts_main (
    id              UUID DEFAULT gen_random_uuid() PRIMARY KEY,
    name            VARCHAR(64) NOT NULL,
    product         VARCHAR(64) NOT NULL,
    dimensions      JSON,
    ts_slack        VARCHAR(255),
    ts_alert        TIMESTAMP,
    duration        INTEGER,
    sent            BOOLEAN,
    critical        BOOLEAN,
    rca_metadata    JSONB,
    metadata        JSON,
    last_updated_at TIMESTAMP DEFAULT date_trunc('second', CURRENT_TIMESTAMP)
);

CREATE TABLE IF NOT EXISTS alerts_main_xyne (
    id              UUID DEFAULT gen_random_uuid() PRIMARY KEY,
    name            VARCHAR(64) NOT NULL,
    product         VARCHAR(64) NOT NULL,
    dimensions      JSON,
    ts_slack        VARCHAR(255),
    ts_alert        TIMESTAMP,
    duration        INTEGER,
    sent            BOOLEAN,
    critical        BOOLEAN,
    rca_metadata    JSONB,
    metadata        JSON,
    last_updated_at TIMESTAMP DEFAULT date_trunc('second', CURRENT_TIMESTAMP)
);

-- Current lifecycle state: what is firing now, since when, and what recovered.
-- This is what the classifier diffs against; `alerts_main` is the log of what
-- was said about it.
CREATE TABLE IF NOT EXISTS alerts_intermediate (
    id_intermediate        UUID DEFAULT gen_random_uuid() PRIMARY KEY,
    id                     UUID REFERENCES alerts_main(id) ON DELETE CASCADE,
    name                   VARCHAR(64) NOT NULL,
    product                VARCHAR(64) NOT NULL,
    dimensions             JSONB,
    ts_slack               VARCHAR(255),
    ts_alert               TIMESTAMP,
    latest_ts_alert        TIMESTAMP,
    max_duration           INTEGER,
    other_metrics          JSONB,
    metadata               JSONB,
    metadata_alert_details JSONB,
    rca_metadata           JSONB,
    group_id               VARCHAR(64) NOT NULL DEFAULT '',
    priority               VARCHAR(64),
    last_updated_at        TIMESTAMP,
    recovered_ts           TIMESTAMP
);

CREATE TABLE IF NOT EXISTS alerts_intermediate_xyne (
    id_intermediate        UUID DEFAULT gen_random_uuid() PRIMARY KEY,
    id                     UUID REFERENCES alerts_main_xyne(id) ON DELETE CASCADE,
    name                   VARCHAR(64) NOT NULL,
    product                VARCHAR(64) NOT NULL,
    dimensions             JSONB,
    ts_slack               VARCHAR(255),
    ts_alert               TIMESTAMP,
    latest_ts_alert        TIMESTAMP,
    max_duration           INTEGER,
    other_metrics          JSONB,
    metadata               JSONB,
    metadata_alert_details JSONB,
    rca_metadata           JSONB,
    group_id               VARCHAR(64) NOT NULL DEFAULT '',
    priority               VARCHAR(64),
    last_updated_at        TIMESTAMP,
    recovered_ts           TIMESTAMP
);

-- The mappers screen. product and values_ are `json` rather than `jsonb`: the
-- dashboard sends them already serialised and reads them back expecting the
-- same bytes.
CREATE TABLE IF NOT EXISTS alerts_dicts (
    id         UUID DEFAULT gen_random_uuid() PRIMARY KEY,
    name       VARCHAR(64) NOT NULL,
    key_       VARCHAR(255) NOT NULL,
    product    JSON,
    values_    JSON,
    ts_created TIMESTAMP,
    is_enabled BOOLEAN DEFAULT TRUE,
    username   VARCHAR(64) DEFAULT 'reliability_team',
    metadata   JSON
);

-- One enabled entry per name and key; superseded rows stay for history.
CREATE UNIQUE INDEX IF NOT EXISTS idx_alerts_dicts_enabled_unique
    ON alerts_dicts USING btree (name, key_) WHERE is_enabled IS TRUE;

CREATE INDEX IF NOT EXISTS idx_alerts_dicts_latest
    ON alerts_dicts USING btree (name, key_, ts_created DESC);

-- Per-merchant alert instances. current_metric against expected_metric is the
-- generic form of "what is wrong", so a detector that is not about success rate
-- needs no schema change.
CREATE TABLE IF NOT EXISTS merchants_alert_external (
    id                     UUID REFERENCES alerts_main(id) ON DELETE CASCADE,
    id_merchant_table      UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    id_intermediate        UUID,
    name                   VARCHAR(64) NOT NULL,
    product                VARCHAR(64) NOT NULL,
    merchant_id            VARCHAR(64) NOT NULL,
    dimensions             JSONB,
    auxiliary_dimensions   JSONB,
    current_metric         DOUBLE PRECISION NOT NULL,
    expected_metric        DOUBLE PRECISION NOT NULL,
    attribution            VARCHAR(255),
    max_duration           INTEGER NOT NULL,
    start_time             TIMESTAMP NOT NULL,
    is_visible             BOOLEAN NOT NULL DEFAULT TRUE,
    recovered_ts           TIMESTAMP,
    ts_slack               VARCHAR(255) NOT NULL,
    ts_alert               TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    latest_ts_alert        TIMESTAMP,
    last_updated_at        TIMESTAMP,
    slack_info             JSONB,
    communication_info     JSONB,
    metadata               JSONB,
    metadata_alert_details JSONB,
    priority               VARCHAR(64),
    tenant_id              VARCHAR(64)
);

CREATE INDEX IF NOT EXISTS idx_alerts_external
    ON merchants_alert_external (merchant_id);
CREATE INDEX IF NOT EXISTS idx_ts_alerts_external
    ON merchants_alert_external USING btree (ts_alert);

CREATE TABLE IF NOT EXISTS merchants_alert_external_xyne (
    id                     UUID REFERENCES alerts_main_xyne(id) ON DELETE CASCADE,
    id_merchant_table      UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    id_intermediate        UUID,
    name                   VARCHAR(64) NOT NULL,
    product                VARCHAR(64) NOT NULL,
    merchant_id            VARCHAR(64) NOT NULL,
    dimensions             JSONB,
    auxiliary_dimensions   JSONB,
    current_metric         DOUBLE PRECISION NOT NULL,
    expected_metric        DOUBLE PRECISION NOT NULL,
    attribution            VARCHAR(255),
    max_duration           INTEGER NOT NULL,
    start_time             TIMESTAMP NOT NULL,
    is_visible             BOOLEAN NOT NULL DEFAULT TRUE,
    recovered_ts           TIMESTAMP,
    ts_slack               VARCHAR(255) NOT NULL,
    ts_alert               TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    latest_ts_alert        TIMESTAMP,
    last_updated_at        TIMESTAMP,
    slack_info             JSONB,
    communication_info     JSONB,
    metadata               JSONB,
    metadata_alert_details JSONB,
    priority               VARCHAR(64),
    tenant_id              VARCHAR(64)
);

CREATE INDEX IF NOT EXISTS idx_alerts_external_xyne
    ON merchants_alert_external_xyne (merchant_id);
CREATE INDEX IF NOT EXISTS idx_ts_alerts_external_xyne
    ON merchants_alert_external_xyne USING btree (ts_alert);

-- One row per dimension of an instance, so a single alert can be broken down by
-- connector, method or anything else without widening the instance table.
CREATE TABLE IF NOT EXISTS merchants_alert_external_dimension (
    id                     UUID REFERENCES alerts_main(id) ON DELETE CASCADE,
    id_merchant_table      UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    id_intermediate        UUID,
    name                   VARCHAR(64) NOT NULL,
    product                VARCHAR(64) NOT NULL,
    dimension_key          VARCHAR(64) NOT NULL,
    dimension_value        VARCHAR(255) NOT NULL,
    dimensions             JSONB,
    auxiliary_dimensions   JSONB,
    current_metric         DOUBLE PRECISION NOT NULL,
    expected_metric        DOUBLE PRECISION NOT NULL,
    attribution            VARCHAR(255),
    max_duration           INTEGER NOT NULL,
    is_visible             BOOLEAN NOT NULL DEFAULT TRUE,
    start_time             TIMESTAMP NOT NULL,
    recovered_ts           TIMESTAMP,
    ts_slack               VARCHAR(255),
    ts_alert               TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    latest_ts_alert        TIMESTAMP,
    last_updated_at        TIMESTAMP,
    slack_info             JSONB,
    communication_info     JSONB,
    metadata               JSONB,
    metadata_alert_details JSONB,
    priority               VARCHAR(64),
    tenant_id              VARCHAR(64)
);

CREATE INDEX IF NOT EXISTS idx_alerts_external_dimension
    ON merchants_alert_external_dimension (name, product, dimension_key);

-- Which alerts are on, per name and product.
--
-- (name, product) is the natural key: without it two rows can disagree about
-- whether the same alert is enabled. Diesel also refuses to model a table with
-- no primary key, and this table has to be readable.
CREATE TABLE IF NOT EXISTS merchants_alert_external_config (
    name            VARCHAR(64) NOT NULL,
    product         VARCHAR(64) NOT NULL,
    category        VARCHAR(64) DEFAULT '',
    is_enabled      BOOLEAN DEFAULT TRUE,
    metadata        JSONB DEFAULT '{}',
    last_updated_at TIMESTAMP,
    PRIMARY KEY (name, product)
);

-- Notification bell read watermarks. Keyed by user, though with authentication
-- disabled every read arrives under the same empty name and there is one row.
CREATE TABLE IF NOT EXISTS notification_reads (
    user_name    VARCHAR(255) PRIMARY KEY,
    last_read_at TIMESTAMP NOT NULL
);
