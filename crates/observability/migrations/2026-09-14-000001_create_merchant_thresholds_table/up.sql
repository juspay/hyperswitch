CREATE TABLE IF NOT EXISTS merchant_thresholds (
    id                             UUID PRIMARY KEY,
    name                           VARCHAR(64) NOT NULL,
    product                        VARCHAR(64) NOT NULL,
    merchant_id                    VARCHAR(64) NOT NULL,
    thresholds_min_volume          DOUBLE PRECISION,
    thresholds_min_impacted_volume DOUBLE PRECISION,
    thresholds_tolerance           DOUBLE PRECISION,
    thresholds_diff_threshold      DOUBLE PRECISION,
    thresholds_merchant_impact     DOUBLE PRECISION,
    thresholds_alert_period        DOUBLE PRECISION,
    thresholds_min_observations    DOUBLE PRECISION,
    thresholds_min_history_volume  DOUBLE PRECISION,
    thresholds_filter_percentile   DOUBLE PRECISION,
    thresholds_current_min_volume  DOUBLE PRECISION,
    metadata                       JSONB,
    author                         VARCHAR(64) NOT NULL,
    is_enabled                     BOOLEAN NOT NULL,
    last_updated_at                TIMESTAMP NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS merchant_thresholds_unique_index
    ON merchant_thresholds (name, product, merchant_id, is_enabled, author);
