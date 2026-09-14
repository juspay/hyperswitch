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
    id               UUID PRIMARY KEY,
    name             VARCHAR(64) NOT NULL,
    product          VARCHAR(64) NOT NULL,
    dimensions       VARCHAR(255) NOT NULL,
    period           INTEGER NOT NULL,
    default_channel  VARCHAR(64),
    default_critical BOOLEAN NOT NULL,
    blacklist        JSON,
    snooze           JSON,
    history_window   INTEGER,
    thresholds       JSON,
    metadata         JSON,
    is_enabled       BOOLEAN NOT NULL,
    comments         JSON,
    call_period      INTEGER,
    author           VARCHAR(64) NOT NULL,
    approver         VARCHAR(64),
    last_updated_at  TIMESTAMP NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS alerts_info_name_product_index
    ON alerts_info USING btree (name, product);

-- Announcements actually sent, one row per alert per delivery. `sent` and
-- `ts_slack` are the record that an alert reached a channel; without them a
-- failed delivery is indistinguishable from a successful one on the next run.
CREATE TABLE IF NOT EXISTS alerts_main (
    id              UUID PRIMARY KEY,
    channel         VARCHAR(64) NOT NULL,
    name            VARCHAR(64),
    product         VARCHAR(64),
    dimensions      JSON NOT NULL,
    ts_slack        VARCHAR(255),
    ts_alert        TIMESTAMP NOT NULL,
    duration        INTEGER NOT NULL,
    sent            BOOLEAN NOT NULL,
    critical        BOOLEAN NOT NULL,
    rca_metadata    JSONB NOT NULL,
    metadata        JSON,
    last_updated_at TIMESTAMP NOT NULL
);

CREATE INDEX IF NOT EXISTS alerts_main_channel_ts_alert_index
    ON alerts_main USING btree (channel, ts_alert);
CREATE INDEX IF NOT EXISTS alerts_main_last_updated_at_index
    ON alerts_main USING btree (last_updated_at);

-- Current lifecycle state: what is firing now, since when, and what recovered.
-- This is what the classifier diffs against; `alerts_main` is the log of what
-- was said about it.
CREATE TABLE IF NOT EXISTS alerts_intermediate (
    id_intermediate        UUID PRIMARY KEY,
    channel                VARCHAR(64) NOT NULL,
    id                     UUID REFERENCES alerts_main(id) ON DELETE CASCADE,
    name                   VARCHAR(64),
    product                VARCHAR(64),
    dimensions             JSONB NOT NULL,
    ts_slack               VARCHAR(255),
    ts_alert               TIMESTAMP NOT NULL,
    latest_ts_alert        TIMESTAMP NOT NULL,
    max_duration           INTEGER NOT NULL,
    other_metrics          JSONB,
    metadata               JSONB,
    metadata_alert_details JSONB,
    rca_metadata           JSONB NOT NULL,
    group_id               VARCHAR(64) NOT NULL,
    priority               VARCHAR(64) NOT NULL,
    last_updated_at        TIMESTAMP NOT NULL,
    recovered_ts           TIMESTAMP
);

CREATE INDEX IF NOT EXISTS alerts_intermediate_channel_ts_alert_latest_ts_alert_index
    ON alerts_intermediate USING btree (channel, ts_alert, latest_ts_alert);
CREATE INDEX IF NOT EXISTS alerts_intermediate_id_index
    ON alerts_intermediate USING btree (id);

-- The mappers screen. product and values_ are `json` rather than `jsonb`: the
-- dashboard sends them already serialised and reads them back expecting the
-- same bytes.
CREATE TABLE IF NOT EXISTS alerts_dicts (
    id         UUID PRIMARY KEY,
    name       VARCHAR(64) NOT NULL,
    key_       VARCHAR(255) NOT NULL,
    product    JSON NOT NULL,
    values_    JSON NOT NULL,
    ts_created TIMESTAMP NOT NULL,
    is_enabled BOOLEAN NOT NULL,
    username   VARCHAR(64) NOT NULL,
    metadata   JSON NOT NULL
);

-- One enabled entry per name and key; superseded rows stay for history.
CREATE UNIQUE INDEX IF NOT EXISTS alerts_dicts_name_key_enabled_index
    ON alerts_dicts USING btree (name, key_) WHERE is_enabled IS TRUE;

CREATE INDEX IF NOT EXISTS alerts_dicts_name_key_ts_created_index
    ON alerts_dicts USING btree (name, key_, ts_created DESC);

-- Per-merchant alert instances. current_metric against expected_metric is the
-- generic form of "what is wrong", so a detector that is not about success rate
-- needs no schema change.
CREATE TABLE IF NOT EXISTS merchants_alert_external (
    id                     UUID NOT NULL REFERENCES alerts_main(id) ON DELETE CASCADE,
    channel                VARCHAR(64) NOT NULL,
    id_merchant_table      UUID PRIMARY KEY,
    id_intermediate        UUID,
    name                   VARCHAR(64) NOT NULL,
    product                VARCHAR(64) NOT NULL,
    merchant_id            VARCHAR(64) NOT NULL,
    dimensions             JSONB NOT NULL,
    auxiliary_dimensions   JSONB NOT NULL,
    current_metric         DOUBLE PRECISION,
    expected_metric        DOUBLE PRECISION,
    attribution            VARCHAR(255) NOT NULL,
    max_duration           INTEGER,
    start_time             TIMESTAMP,
    is_visible             BOOLEAN NOT NULL,
    recovered_ts           TIMESTAMP,
    ts_slack               VARCHAR(255) NOT NULL,
    ts_alert               TIMESTAMP NOT NULL,
    latest_ts_alert        TIMESTAMP,
    last_updated_at        TIMESTAMP NOT NULL,
    slack_info             JSONB NOT NULL,
    communication_info     JSONB NOT NULL,
    metadata               JSONB NOT NULL,
    metadata_alert_details JSONB NOT NULL,
    priority               VARCHAR(64) NOT NULL,
    tenant_id              VARCHAR(64) NOT NULL
);

CREATE INDEX IF NOT EXISTS merchants_alert_external_channel_merchant_id_index
    ON merchants_alert_external (channel, merchant_id);
CREATE INDEX IF NOT EXISTS merchants_alert_external_ts_alert_index
    ON merchants_alert_external USING btree (ts_alert);
CREATE INDEX IF NOT EXISTS merchants_alert_external_id_index
    ON merchants_alert_external USING btree (id);

-- One row per dimension of an instance, so a single alert can be broken down by
-- connector, method or anything else without widening the instance table.
CREATE TABLE IF NOT EXISTS merchants_alert_external_dimension (
    id                     UUID NOT NULL REFERENCES alerts_main(id) ON DELETE CASCADE,
    channel                VARCHAR(64) NOT NULL,
    id_merchant_table      UUID PRIMARY KEY,
    id_intermediate        UUID,
    name                   VARCHAR(64) NOT NULL,
    product                VARCHAR(64) NOT NULL,
    dimension_key          VARCHAR(64) NOT NULL,
    dimension_value        VARCHAR(255) NOT NULL,
    dimensions             JSONB NOT NULL,
    auxiliary_dimensions   JSONB NOT NULL,
    current_metric         DOUBLE PRECISION,
    expected_metric        DOUBLE PRECISION,
    attribution            VARCHAR(255) NOT NULL,
    max_duration           INTEGER,
    is_visible             BOOLEAN NOT NULL,
    start_time             TIMESTAMP,
    recovered_ts           TIMESTAMP,
    ts_slack               VARCHAR(255) NOT NULL,
    ts_alert               TIMESTAMP NOT NULL,
    latest_ts_alert        TIMESTAMP,
    last_updated_at        TIMESTAMP NOT NULL,
    slack_info             JSONB NOT NULL,
    communication_info     JSONB NOT NULL,
    metadata               JSONB NOT NULL,
    metadata_alert_details JSONB NOT NULL,
    priority               VARCHAR(64) NOT NULL,
    tenant_id              VARCHAR(64) NOT NULL
);

CREATE INDEX IF NOT EXISTS merchants_alert_external_dimension_value_key_index
    ON merchants_alert_external_dimension (dimension_value, dimension_key);
CREATE INDEX IF NOT EXISTS merchants_alert_external_dimension_ts_alert_index
    ON merchants_alert_external_dimension USING btree (ts_alert);
CREATE INDEX IF NOT EXISTS merchants_alert_external_dimension_id_index
    ON merchants_alert_external_dimension USING btree (id);

-- Which alerts are on, per name and product.
--
-- (name, product) is the natural key: without it two rows can disagree about
-- whether the same alert is enabled. Diesel also refuses to model a table with
-- no primary key, and this table has to be readable.
CREATE TABLE IF NOT EXISTS merchants_alert_external_config (
    name            VARCHAR(64) NOT NULL,
    product         VARCHAR(64) NOT NULL,
    category        VARCHAR(64) NOT NULL,
    is_enabled      BOOLEAN NOT NULL,
    metadata        JSONB NOT NULL,
    last_updated_at TIMESTAMP NOT NULL,
    PRIMARY KEY (name, product)
);

-- Notification bell read watermarks. Keyed by user, though with authentication
-- disabled every read arrives under the same empty name and there is one row.
CREATE TABLE IF NOT EXISTS notification_reads (
    user_name    VARCHAR(255) PRIMARY KEY,
    last_read_at TIMESTAMP NOT NULL
);
