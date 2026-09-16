CREATE TABLE success_rate_threshold_overrides (
    name                  TEXT NOT NULL,
    product               TEXT NOT NULL,
    merchant_id           TEXT NOT NULL,
    profile_id            TEXT NOT NULL DEFAULT '',
    min_volume            DOUBLE PRECISION,
    min_impacted_volume   DOUBLE PRECISION,
    tolerance             DOUBLE PRECISION,
    diff_threshold        DOUBLE PRECISION,
    updated_by            TEXT NOT NULL,
    last_updated_at       TIMESTAMP NOT NULL
                          DEFAULT date_trunc('second', CURRENT_TIMESTAMP),
    is_deleted            BOOLEAN NOT NULL DEFAULT FALSE,

    PRIMARY KEY (name, product, merchant_id, profile_id),

    CHECK (merchant_id = btrim(merchant_id) AND merchant_id <> ''),
    CHECK (profile_id = btrim(profile_id))
);
