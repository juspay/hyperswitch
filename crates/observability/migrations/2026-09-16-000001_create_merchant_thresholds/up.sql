CREATE TABLE IF NOT EXISTS merchant_thresholds (
    id                             UUID DEFAULT gen_random_uuid() PRIMARY KEY,
    name                           VARCHAR(64),
    product                        VARCHAR(64),
    merchant_id                    VARCHAR(64),
    profile_id                     VARCHAR(64) NOT NULL DEFAULT '',
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
    author                         VARCHAR(64) DEFAULT 'reliability_team',
    is_enabled                     BOOLEAN DEFAULT FALSE,
    last_updated_at                TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT merchant_thresholds_conflict
        UNIQUE (name, product, merchant_id, profile_id, is_enabled, author)
);
